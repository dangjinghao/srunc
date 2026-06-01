//! `authu` — srun (深澜) campus-portal CLI.
//!
//! Subcommands: `login` / `logout` / `status` / `keepalive`.
//!
//! The hard, side-effect-free crypto lives in `srun-core`; the HTTP client and
//! Tsinghua-specific `ac_id` scraping live in `srun`. This binary is the thin
//! imperative shell on top.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use clap::{Parser, Subcommand};
use serde::Deserialize;

use srun::{Client, Endpoints};
use srun_core::Credentials;

// ---------- CLI ----------

#[derive(Parser, Debug)]
#[command(
    name = "authu",
    version,
    about = "srun (深澜) campus-portal auth tool",
    long_about = "Authenticate against srun-based campus captive portals. \
                  Defaults to Tsinghua's auth4/auth6 endpoints, but `--host` \
                  works for any srun deployment."
)]
struct Cli {
    // ---- identity ----
    /// Username (overrides config).
    #[arg(short, long, global = true)]
    username: Option<String>,
    /// Password (overrides config).
    #[arg(short, long, global = true)]
    password: Option<String>,

    /// Config file path.
    /// Default search: $AUTHU_CONFIG, $XDG_CONFIG_HOME/authu/config.toml,
    /// ~/.config/authu/config.toml, ~/.authu.toml.
    #[arg(short, long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    // ---- connection ----
    /// Override portal hostname (default: auth4/auth6.tsinghua.edu.cn).
    #[arg(long, global = true)]
    host: Option<String>,
    /// Use the IPv6 portal (auth6.tsinghua.edu.cn).
    #[arg(short = '6', long, global = true)]
    ipv6: bool,
    /// Use http:// instead of https://.
    #[arg(long, global = true)]
    insecure: bool,
    /// HTTP request timeout, in seconds.
    #[arg(short = 't', long, global = true, value_name = "SECS")]
    timeout: Option<u64>,

    // ---- auth scoping ----
    /// Use a specific ac_id (skips detection).
    #[arg(long, global = true)]
    ac_id: Option<String>,
    /// Authenticate for a specific IP address (proxy mode).
    #[arg(long, global = true)]
    ip: Option<String>,

    // ---- service mode ----
    /// Don't read from stdin and only emit ERROR-level logs. For systemd / init.d.
    #[arg(short = 'D', long, global = true)]
    daemonize: bool,

    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand, Debug)]
enum Cmd {
    /// Authenticate to the portal.
    Login {
        /// Keep the session alive after a successful login.
        #[arg(short, long)]
        keepalive: bool,
        /// Probe an in-campus URL (not the public internet) for keepalive.
        #[arg(short = 'C', long)]
        campus_only: bool,
        /// Seconds between keepalive probes.
        #[arg(long, default_value_t = 3)]
        interval: u64,
        /// Consecutive probe failures before keepalive gives up.
        #[arg(long, default_value_t = 2)]
        retry: u32,
    },
    /// De-authenticate.
    Logout,
    /// Print whether we are currently online.
    Status,
    /// Run only the keepalive loop (no login attempt).
    Keepalive {
        /// Probe an in-campus URL instead of the public internet.
        #[arg(short = 'C', long)]
        campus_only: bool,
        #[arg(short, long, default_value_t = 3)]
        interval: u64,
        #[arg(short, long, default_value_t = 2)]
        retry: u32,
    },
}

// ---------- config + settings ----------

#[derive(Deserialize, Default, Debug)]
#[serde(default, deny_unknown_fields)]
struct ConfigFile {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    ipv6: bool,
    insecure: bool,
    ac_id: Option<String>,
    ip: Option<String>,
    timeout: Option<u64>,
    daemonize: bool,
}

#[derive(Debug)]
struct Settings {
    username: String,
    password: String,
    host: Option<String>,
    ipv6: bool,
    insecure: bool,
    ac_id: Option<String>,
    ip: String,
    timeout: Duration,
    daemonize: bool,
}

fn locate_config(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("AUTHU_CONFIG") {
        return Some(p.into());
    }
    let join_exists = |base: &str, sub: &str| -> Option<PathBuf> {
        let p = Path::new(base).join(sub);
        p.exists().then_some(p)
    };
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if let Some(p) = join_exists(&xdg, "authu/config.toml") {
            return Some(p);
        }
    }
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())?;
    join_exists(&home, ".config/authu/config.toml").or_else(|| join_exists(&home, ".authu.toml"))
}

fn resolve(cli: &Cli) -> Result<Settings> {
    let cfg: ConfigFile = match locate_config(cli.config.as_deref()) {
        Some(p) => {
            let text = fs::read_to_string(&p)
                .with_context(|| format!("reading config file {}", p.display()))?;
            toml::from_str(&text).with_context(|| format!("parsing config file {}", p.display()))?
        }
        None => ConfigFile::default(),
    };

    Ok(Settings {
        username: cli.username.clone().or(cfg.username).unwrap_or_default(),
        password: cli.password.clone().or(cfg.password).unwrap_or_default(),
        host: cli.host.clone().or(cfg.host),
        ipv6: cli.ipv6 || cfg.ipv6,
        insecure: cli.insecure || cfg.insecure,
        ac_id: cli.ac_id.clone().or(cfg.ac_id),
        ip: cli.ip.clone().or(cfg.ip).unwrap_or_default(),
        timeout: Duration::from_secs(cli.timeout.or(cfg.timeout).unwrap_or(2)),
        daemonize: cli.daemonize || cfg.daemonize,
    })
}

fn prompt_credentials(s: &mut Settings) -> Result<()> {
    if !s.daemonize {
        if s.username.is_empty() {
            print!("Username: ");
            std::io::stdout().flush()?;
            let mut buf = String::new();
            std::io::stdin().read_line(&mut buf)?;
            s.username = buf.trim().to_string();
        }
        if s.password.is_empty() {
            s.password = rpassword::prompt_password("Password: ")?;
        }
    }
    if s.username.is_empty() {
        bail!("username required (set in config, pass --username, or run without --daemonize)");
    }
    Ok(())
}

fn make_endpoints(s: &Settings) -> Endpoints {
    if let Some(h) = &s.host {
        Endpoints::from_host(h, s.insecure)
    } else if s.ipv6 {
        Endpoints::tsinghua_v6(s.insecure)
    } else {
        Endpoints::tsinghua_v4(s.insecure)
    }
}

fn resolve_ac_id(client: &Client, s: &Settings) -> String {
    if let Some(v) = &s.ac_id {
        return v.clone();
    }
    match srun::tsinghua::detect_ac_id(client, s.ipv6) {
        Ok(v) => v,
        Err(e) => {
            if !s.daemonize {
                eprintln!("warn: ac_id detection failed ({e}); falling back to \"1\"");
            }
            "1".into()
        }
    }
}

// ---------- keepalive ----------

fn install_signal_handler() -> Result<Arc<AtomicBool>> {
    let stop = Arc::new(AtomicBool::new(false));
    {
        let stop = stop.clone();
        ctrlc::set_handler(move || stop.store(true, Ordering::SeqCst))
            .context("installing signal handler")?;
    }
    Ok(stop)
}

fn run_keepalive(
    stop: Arc<AtomicBool>,
    interval: u64,
    retry: u32,
    ipv6: bool,
    campus_only: bool,
    daemonize: bool,
) -> Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    let target = if campus_only || ipv6 {
        "https://www.tsinghua.edu.cn/"
    } else {
        "https://www.baidu.com/"
    };
    if !daemonize {
        eprintln!("keepalive: probing {target} every {interval}s");
    }

    let mut fails = 0u32;
    while !stop.load(Ordering::SeqCst) {
        match agent.head(target).call() {
            Ok(_) => fails = 0,
            Err(e) => {
                fails += 1;
                if fails >= retry {
                    bail!("keepalive: probe failed {fails} times in a row; latest: {e}");
                }
                if !daemonize {
                    eprintln!("keepalive: probe failed ({fails}/{retry}): {e}");
                }
            }
        }
        // Sleep in 1-second slices so SIGTERM is responsive.
        let mut left = interval;
        while left > 0 && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(1));
            left -= 1;
        }
    }
    if !daemonize {
        eprintln!("keepalive: stop signal received, exiting");
    }
    Ok(())
}

// ---------- subcommands ----------

fn cmd_login(
    cli: &Cli,
    keepalive: bool,
    campus_only: bool,
    interval: u64,
    retry: u32,
) -> Result<()> {
    let mut s = resolve(cli)?;
    prompt_credentials(&mut s)?;

    let client = Client::new(make_endpoints(&s), s.timeout);
    let ac_id = resolve_ac_id(&client, &s);
    let creds = Credentials {
        username: &s.username,
        password: &s.password,
    };

    client
        .login(&creds, &ac_id, &s.ip)
        .map_err(|e| anyhow!("login failed: {e}"))?;
    if !s.daemonize {
        println!("login: ok");
    }

    if keepalive {
        let stop = install_signal_handler()?;
        run_keepalive(stop, interval, retry, s.ipv6, campus_only, s.daemonize)?;
    }
    Ok(())
}

fn cmd_logout(cli: &Cli) -> Result<()> {
    let mut s = resolve(cli)?;
    if s.username.is_empty() && !s.daemonize {
        print!("Username: ");
        std::io::stdout().flush()?;
        let mut buf = String::new();
        std::io::stdin().read_line(&mut buf)?;
        s.username = buf.trim().to_string();
    }
    if s.username.is_empty() {
        bail!("username required");
    }
    let client = Client::new(make_endpoints(&s), s.timeout);
    let ac_id = resolve_ac_id(&client, &s);
    client
        .logout(&s.username, &ac_id, &s.ip)
        .map_err(|e| anyhow!("logout failed: {e}"))?;
    if !s.daemonize {
        println!("logout: ok");
    }
    Ok(())
}

fn cmd_status(cli: &Cli) -> Result<()> {
    let s = resolve(cli)?;
    let client = Client::new(make_endpoints(&s), s.timeout);
    let ac_id = resolve_ac_id(&client, &s);
    let st = client
        .status(&ac_id)
        .map_err(|e| anyhow!("status query failed: {e}"))?;
    if st.online {
        println!(
            "online (user={}, ip={})",
            st.username.as_deref().unwrap_or("?"),
            st.ip.as_deref().unwrap_or("?")
        );
    } else {
        println!("offline");
    }
    Ok(())
}

fn cmd_keepalive(cli: &Cli, campus_only: bool, interval: u64, retry: u32) -> Result<()> {
    let s = resolve(cli)?;
    let stop = install_signal_handler()?;
    run_keepalive(stop, interval, retry, s.ipv6, campus_only, s.daemonize)
}

// ---------- entry ----------

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match &cli.cmd {
        Cmd::Login {
            keepalive,
            campus_only,
            interval,
            retry,
        } => cmd_login(&cli, *keepalive, *campus_only, *interval, *retry),
        Cmd::Logout => cmd_logout(&cli),
        Cmd::Status => cmd_status(&cli),
        Cmd::Keepalive {
            campus_only,
            interval,
            retry,
        } => cmd_keepalive(&cli, *campus_only, *interval, *retry),
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}
