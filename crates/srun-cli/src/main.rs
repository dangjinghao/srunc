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
use log::{debug, info, warn, LevelFilter};
use serde::Deserialize;

use srun_client::{Client, Endpoints};
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
    /// Override portal hostname. If omitted, the host is auto-discovered from
    /// the captive-portal redirect (DNS-free, by probing the default gateway);
    /// falls back to auth4/auth6.tsinghua.edu.cn.
    #[arg(long, global = true)]
    host: Option<String>,
    /// Use the IPv6 portal (only affects the Tsinghua fallback default).
    #[arg(short = '6', long, global = true)]
    ipv6: bool,
    /// Use http:// instead of https://.
    #[arg(long, global = true)]
    insecure: bool,
    /// HTTP request timeout, in seconds.
    #[arg(short = 't', long, global = true, value_name = "SECS")]
    timeout: Option<u64>,

    // ---- auth scoping ----
    /// Use a specific ac_id (skips auto-discovery).
    #[arg(long, global = true)]
    ac_id: Option<String>,
    /// Authenticate for a specific IP address (proxy mode).
    #[arg(long, global = true)]
    ip: Option<String>,

    // ---- output ----
    /// Emit machine-readable JSON on success (currently affects `status`).
    #[arg(long, global = true)]
    json: bool,

    /// Increase log verbosity (`-v` = debug, `-vv` = trace).
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    verbose: u8,
    /// Decrease log verbosity (`-q` = warn-only, `-qq` = error-only, `-qqq` = silent).
    #[arg(short = 'q', long, global = true, action = clap::ArgAction::Count)]
    quiet: u8,

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
            basic_toml::from_str(&text)
                .with_context(|| format!("parsing config file {}", p.display()))?
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

/// Resolve the `{endpoints, ac_id}` pair, auto-discovering missing fields.
///
/// Fallback chain (first non-None wins for each):
///
/// 1. Explicit `--host` / `--ac-id` (CLI or TOML).
/// 2. **Captive-portal discovery**: probe `http://<default-gateway>/` (DNS-free)
///    and read both the auth host AND the ac_id out of the srun redirect.
///    This is deployment-agnostic — works for any srun campus and survives
///    routers booting before DNS / auth.
/// 3. Tsinghua default host (`auth4`/`auth6.tsinghua.edu.cn`, picked by
///    `--ipv6`) + legacy DNS-based `tsinghua::detect_ac_id` for ac_id.
/// 4. Final fallback ac_id `"1"` (with a warning unless daemonized).
fn resolve_endpoints_and_ac_id(s: &Settings) -> (Endpoints, String) {
    let need_discovery = s.host.is_none() || s.ac_id.is_none();
    let discovered = if need_discovery {
        srun_client::gateway::default_gateway_v4()
            .and_then(|gw| {
                info!("probing gateway {gw} for srun portal hints");
                match srun_client::discover(&gw, s.timeout) {
                    Ok(d) => {
                        debug!("discovered: host={:?} ac_id={:?}", d.host, d.ac_id);
                        Some(d)
                    }
                    Err(e) => {
                        debug!("gateway probe failed: {e}");
                        None
                    }
                }
            })
            .unwrap_or_default()
    } else {
        Default::default()
    };

    let host = s.host.clone().or(discovered.host).unwrap_or_else(|| {
        if s.ipv6 {
            "auth6.tsinghua.edu.cn".into()
        } else {
            "auth4.tsinghua.edu.cn".into()
        }
    });
    debug!("using portal host: {host}");
    let endpoints = Endpoints::from_host(&host, s.insecure);

    let ac_id = s
        .ac_id
        .clone()
        .or(discovered.ac_id)
        .or_else(|| {
            let client = Client::new(endpoints.clone(), s.timeout);
            srun_client::tsinghua::detect_ac_id(&client, s.ipv6).ok()
        })
        .unwrap_or_else(|| {
            warn!("ac_id detection failed; falling back to \"1\"");
            "1".into()
        });
    debug!("using ac_id: {ac_id}");

    (endpoints, ac_id)
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
    let _ = daemonize; // level filtering is now handled by the logger, not ad-hoc gating
    info!("keepalive: probing {target} every {interval}s");

    let mut fails = 0u32;
    while !stop.load(Ordering::SeqCst) {
        match agent.head(target).call() {
            Ok(_) => fails = 0,
            Err(e) => {
                fails += 1;
                if fails >= retry {
                    bail!("keepalive: probe failed {fails} times in a row; latest: {e}");
                }
                warn!("keepalive: probe failed ({fails}/{retry}): {e}");
            }
        }
        // Sleep in 1-second slices so SIGTERM is responsive.
        let mut left = interval;
        while left > 0 && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(1));
            left -= 1;
        }
    }
    info!("keepalive: stop signal received, exiting");
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

    let (endpoints, ac_id) = resolve_endpoints_and_ac_id(&s);
    let client = Client::new(endpoints, s.timeout);
    let creds = Credentials {
        username: &s.username,
        password: &s.password,
    };

    client
        .login(&creds, &ac_id, &s.ip)
        .map_err(|e| anyhow!("login failed: {e}"))?;
    info!("login: ok");

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
    let (endpoints, ac_id) = resolve_endpoints_and_ac_id(&s);
    let client = Client::new(endpoints, s.timeout);
    client
        .logout(&s.username, &ac_id, &s.ip)
        .map_err(|e| anyhow!("logout failed: {e}"))?;
    info!("logout: ok");
    Ok(())
}

fn cmd_status(cli: &Cli) -> Result<()> {
    let s = resolve(cli)?;
    let (endpoints, ac_id) = resolve_endpoints_and_ac_id(&s);
    let client = Client::new(endpoints, s.timeout);
    let st = client
        .status(&ac_id)
        .map_err(|e| anyhow!("status query failed: {e}"))?;
    if cli.json {
        println!("{}", serde_json::to_string(&st)?);
    } else if st.online {
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

// ---------- logging ----------

/// Configure and install the global logger.
///
/// Base level: `ERROR` under `--daemonize` (procd / systemd captures stderr
/// into journals — keep them quiet), else `INFO`. `-v` bumps a level, `-q`
/// drops one, and `RUST_LOG` (env_logger's standard spec, supports
/// `module=level` filters) overrides everything.
///
/// Third-party crates (ureq, rustls, …) are clamped to `WARN` regardless,
/// so `-v` debug doesn't drown the user in TLS-handshake chatter. NB: the
/// binary's module path is `authu` (the `[[bin]]` name), not `srun_cli`
/// (the package name), so the whitelist names both roots explicitly.
fn init_logger(verbose: u8, quiet: u8, daemonize: bool) {
    let base: i16 = if daemonize { 1 } else { 3 };
    let n = (base + verbose as i16 - quiet as i16).clamp(0, 5);
    let our_level = match n {
        0 => LevelFilter::Off,
        1 => LevelFilter::Error,
        2 => LevelFilter::Warn,
        3 => LevelFilter::Info,
        4 => LevelFilter::Debug,
        _ => LevelFilter::Trace,
    };

    let mut b = env_logger::Builder::new();
    b.filter_level(LevelFilter::Warn);
    b.filter_module("authu", our_level);
    b.filter_module("srun_client", our_level);
    b.filter_module("srun_core", our_level);
    // RUST_LOG, if set, layers its directives on top — most-specific match wins.
    if let Ok(spec) = std::env::var("RUST_LOG") {
        b.parse_filters(&spec);
    }
    b.format(|buf, record| {
        let style = buf.default_level_style(record.level());
        writeln!(
            buf,
            "{style}{:5}{style:#}  {}",
            record.level(),
            record.args(),
        )
    });
    let _ = b.try_init();
}

// ---------- entry ----------

fn main() -> ExitCode {
    let cli = Cli::parse();
    init_logger(cli.verbose, cli.quiet, cli.daemonize);
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
