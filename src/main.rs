//! `srunc` binary — srun (深澜) captive-portal CLI.
//!
//! Subcommands: `login` / `logout` / `status` / `profiles`.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Parser, Subcommand};
use log::{debug, info, warn, LevelFilter};
use serde::Deserialize;

use srunc::{Client, Credentials, Endpoints};

// ---------- CLI ----------

/// Match the visual style used by anstream elsewhere in the CLI: green for
/// positive headers, yellow for usage, bold for placeholders.
fn cli_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Green.on_default() | Effects::BOLD)
        .usage(AnsiColor::Yellow.on_default() | Effects::BOLD)
        .literal(AnsiColor::Cyan.on_default() | Effects::BOLD)
        .placeholder(AnsiColor::White.on_default() | Effects::BOLD)
}

#[derive(Parser, Debug)]
#[command(
    name = "srunc",
    version,
    about = "srun (深澜) captive-portal auth tool",
    long_about = "Authenticate against srun-based captive portals. Most \
                  deployments work out of the box via captive-redirect \
                  auto-discovery; use `--profile <id>` or `--host <h>` for \
                  the rest.",
    styles = cli_styles()
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
    /// Default search: $SRUNC_CONFIG, $XDG_CONFIG_HOME/srunc/config.toml,
    /// ~/.config/srunc/config.toml, ~/.srunc.toml.
    #[arg(short, long, global = true, value_name = "PATH")]
    config: Option<PathBuf>,

    // ---- connection ----
    /// Override portal hostname. Auto-discovered from the captive-portal
    /// redirect (DNS-free, by probing the default gateway) if omitted; can
    /// also be set via a `--profile` preset.
    #[arg(long, global = true)]
    host: Option<String>,
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
    /// Use a known-deployment profile for default host + ac_id
    /// (see `srunc profiles list`). Lower priority than explicit
    /// `--host` / `--ac-id`; higher than auto-discovery.
    #[arg(long, global = true, value_name = "ID")]
    profile: Option<String>,

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
    /// Manage deployment profiles (built-in host + ac_id for known srun installs).
    Profiles {
        #[command(subcommand)]
        action: ProfilesCmd,
    },
}

#[derive(Subcommand, Debug)]
enum ProfilesCmd {
    /// List all known deployment profiles.
    List {
        /// Emit JSON instead of a human-readable table. Consumed by the
        /// OpenWrt LuCI app to populate its dropdown.
        #[arg(long)]
        json: bool,
    },
}

// ---------- config + settings ----------

#[derive(Deserialize, Default, Debug)]
#[serde(default, deny_unknown_fields)]
struct ConfigFile {
    username: Option<String>,
    password: Option<String>,
    host: Option<String>,
    insecure: bool,
    ac_id: Option<String>,
    ip: Option<String>,
    timeout: Option<u64>,
    daemonize: bool,
    profile: Option<String>,
}

#[derive(Debug)]
struct Settings {
    username: String,
    password: String,
    host: Option<String>,
    insecure: bool,
    ac_id: Option<String>,
    ip: String,
    timeout: Duration,
    daemonize: bool,
    profile: Option<String>,
}

fn locate_config(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(p) = explicit {
        return Some(p.to_path_buf());
    }
    if let Ok(p) = std::env::var("SRUNC_CONFIG") {
        return Some(p.into());
    }
    let join_exists = |base: &str, sub: &str| -> Option<PathBuf> {
        let p = Path::new(base).join(sub);
        p.exists().then_some(p)
    };
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if let Some(p) = join_exists(&xdg, "srunc/config.toml") {
            return Some(p);
        }
    }
    let home = std::env::var("HOME")
        .ok()
        .or_else(|| std::env::var("USERPROFILE").ok())?;
    join_exists(&home, ".config/srunc/config.toml").or_else(|| join_exists(&home, ".srunc.toml"))
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
        insecure: cli.insecure || cfg.insecure,
        ac_id: cli.ac_id.clone().or(cfg.ac_id),
        ip: cli.ip.clone().or(cfg.ip).unwrap_or_default(),
        timeout: Duration::from_secs(cli.timeout.or(cfg.timeout).unwrap_or(2)),
        daemonize: cli.daemonize || cfg.daemonize,
        profile: cli
            .profile
            .clone()
            .or(cfg.profile)
            .filter(|s| !s.is_empty()),
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

struct Resolved {
    host: String,
    endpoints: Endpoints,
    ac_id: String,
}

/// Fallback chain: explicit `--host`/`--ac-id` → `--profile` preset →
/// captive-portal discovery (probe default gateway). Errors out if all three
/// fail to yield a value, rather than silently picking a stale default.
fn resolve_target(s: &Settings) -> Result<Resolved> {
    let profile = s.profile.as_deref().and_then(|id| {
        let p = srunc::profiles::by_id(id);
        if p.is_none() {
            warn!("unknown profile {id:?}; run `srunc profiles list` for valid ids");
        }
        p
    });
    // Label-only profiles (host: None, ac_id: None) flatten to no contribution;
    // resolution falls through to captive-portal discovery for those fields.
    let profile_host = profile.and_then(|p| p.host).map(str::to_string);
    let profile_ac_id = profile.and_then(|p| p.ac_id).map(str::to_string);

    let need_discovery = (s.host.is_none() && profile_host.is_none())
        || (s.ac_id.is_none() && profile_ac_id.is_none());
    let discovered = if need_discovery {
        srunc::gateway::default_gateway_v4()
            .and_then(|gw| {
                info!("probing gateway {gw} for srun portal hints");
                match srunc::discover(&gw, s.timeout) {
                    Ok(d) => Some(d),
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

    let host = s
        .host
        .clone()
        .or(profile_host)
        .or(discovered.host)
        .ok_or_else(|| {
            anyhow!(
                "could not determine portal host — pass --host, --profile <id>, \
                 or ensure the default gateway redirects to a captive portal"
            )
        })?;
    let endpoints = Endpoints::from_host(&host, s.insecure);

    let ac_id = s
        .ac_id
        .clone()
        .or(profile_ac_id)
        .or(discovered.ac_id)
        .ok_or_else(|| {
            anyhow!(
                "could not determine ac_id — pass --ac-id, --profile <id>, \
                 or ensure the captive redirect carries an index_<N>.html / ac_id=<N> hint"
            )
        })?;

    debug!("resolved: host={host} ac_id={ac_id}");
    Ok(Resolved {
        host,
        endpoints,
        ac_id,
    })
}

fn cmd_profiles_list(json: bool) -> Result<()> {
    let profiles = srunc::profiles::PROFILES;
    if json {
        println!("{}", serde_json::to_string(profiles)?);
        return Ok(());
    }
    use anstream::println;
    use anstyle::{AnsiColor, Style};
    let bold = Style::new().bold();
    let dim = Style::new().dimmed();
    let green = Style::new().fg_color(Some(AnsiColor::Green.into()));
    let yellow = Style::new().fg_color(Some(AnsiColor::Yellow.into()));
    if profiles.is_empty() {
        println!("{dim}(no profiles shipped){dim:#}");
        return Ok(());
    }
    // One-line legend up top so the glyphs aren't cryptic.
    println!(
        "{dim}legend:{dim:#} {green}✓{green:#} verified · {yellow}~{yellow:#} label-only \
         (falls through to captive-portal auto-discovery)"
    );
    println!();
    for p in profiles {
        // A profile is "verified" iff *both* host and ac_id are pinned;
        // label-only otherwise. Matches the semantic in profiles.rs.
        let verified = p.host.is_some() && p.ac_id.is_some();
        let (glyph_style, glyph) = if verified {
            (green, "✓")
        } else {
            (yellow, "~")
        };
        println!(
            "{glyph_style}{glyph}{glyph_style:#} {bold}{}{bold:#}",
            p.id
        );
        println!("  {dim}label{dim:#}  {}", p.label);
        println!(
            "  {dim}host{dim:#}   {}",
            p.host.unwrap_or("— (auto-discover) —")
        );
        println!(
            "  {dim}ac_id{dim:#}  {}",
            p.ac_id.unwrap_or("— (auto-discover) —")
        );
    }
    Ok(())
}

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
    campus_only: bool,
    portal_host: &str,
) -> Result<()> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .build();
    // campus_only: probe the portal host itself (always in-campus, never
    // billed). Otherwise: a public anchor.
    let in_campus = format!("http://{portal_host}/");
    let target: &str = if campus_only {
        &in_campus
    } else {
        "https://www.baidu.com/"
    };
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
        // 1-second slices keep SIGTERM responsive.
        let mut left = interval;
        while left > 0 && !stop.load(Ordering::SeqCst) {
            std::thread::sleep(Duration::from_secs(1));
            left -= 1;
        }
    }
    info!("keepalive: stop received, exiting");
    Ok(())
}

fn cmd_login(
    cli: &Cli,
    keepalive: bool,
    campus_only: bool,
    interval: u64,
    retry: u32,
) -> Result<()> {
    let mut s = resolve(cli)?;
    prompt_credentials(&mut s)?;

    let r = resolve_target(&s)?;
    let client = Client::new(r.endpoints, s.timeout);
    let creds = Credentials {
        username: &s.username,
        password: &s.password,
    };

    client
        .login(&creds, &r.ac_id, &s.ip)
        .context("login failed")?;
    info!("login: ok");

    // Best-effort confirmation summary: re-query status and show "logged in
    // as <user> (IP <ip>)". Skipped under --json (which would expect machine
    // output from login) and under --daemonize (an extra HTTP round-trip on
    // every keepalive cycle is wasteful). Errors here don't fail login.
    if !cli.json && !s.daemonize {
        if let Ok(st) = client.status(&r.ac_id) {
            print_login_confirmation(&st);
        }
    }

    if keepalive {
        let stop = install_signal_handler()?;
        run_keepalive(stop, interval, retry, campus_only, &r.host)?;
    }
    Ok(())
}

fn print_login_confirmation(st: &srunc::Status) {
    use anstream::println;
    use anstyle::{AnsiColor, Style};
    let green = Style::new().fg_color(Some(AnsiColor::Green.into())).bold();
    let dim = Style::new().dimmed();
    let user = st.username.as_deref().unwrap_or("?");
    let ip = st.ip.as_deref().unwrap_or("?");
    println!("{green}→ logged in as {green:#}{user} {dim}(IP {ip}){dim:#}");
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
    let r = resolve_target(&s)?;
    let client = Client::new(r.endpoints, s.timeout);
    client
        .logout(&s.username, &r.ac_id, &s.ip)
        .context("logout failed")?;
    info!("logout: ok");
    Ok(())
}

fn cmd_status(cli: &Cli) -> Result<()> {
    let s = resolve(cli)?;
    let r = resolve_target(&s)?;
    let client = Client::new(r.endpoints, s.timeout);
    let st = client.status(&r.ac_id).context("status query failed")?;
    if cli.json {
        println!("{}", serde_json::to_string(&st)?);
    } else {
        print_status_human(&st);
    }
    Ok(())
}

fn print_status_human(st: &srunc::Status) {
    use anstream::println;
    use anstyle::{AnsiColor, Style};
    let green = Style::new().fg_color(Some(AnsiColor::Green.into())).bold();
    let dim = Style::new().dimmed();
    let bold = Style::new().bold();
    if !st.online {
        println!("{dim}○ Offline{dim:#}");
        return;
    }
    println!("{green}● Online{green:#}");
    let row = |k: &str, v: &str| {
        // Two-space prefix, label column padded to 10 so the widest label
        // ("Keepalive") still gets a separator before its value.
        println!("  {dim}{k:<10}{dim:#}{bold}{v}{bold:#}");
    };
    row("User", st.username.as_deref().unwrap_or("?"));
    row("IP", st.ip.as_deref().unwrap_or("?"));
    let Some(d) = &st.details else { return };
    if let Some(plan) = &d.plan {
        row("Plan", plan);
    }
    if let Some(mac) = &d.user_mac {
        row("MAC", mac);
    }
    if let Some(n) = d.online_device_total {
        row("Devices", &n.to_string());
    }
    if let Some(b) = d.sum_bytes {
        row("Used", &format_bytes(b));
    }
    if let Some(s) = d.sum_seconds {
        row("Time", &format_seconds(s));
    }
    if let Some(t) = d.keepalive_time {
        row("Keepalive", &format_ago(t));
    }
}

/// Human-readable byte size — IEC units, 2 decimal places past KiB.
fn format_bytes(n: u64) -> String {
    const KIB: f64 = 1024.0;
    let n = n as f64;
    if n < KIB {
        return format!("{} B", n as u64);
    }
    let units = ["KiB", "MiB", "GiB", "TiB", "PiB"];
    let mut v = n / KIB;
    let mut u = 0;
    while v >= KIB && u + 1 < units.len() {
        v /= KIB;
        u += 1;
    }
    format!("{v:.2} {}", units[u])
}

/// Coarse duration: "Xd Yh", "Yh Zm", or "Zm" depending on magnitude.
fn format_seconds(s: u64) -> String {
    let d = s / 86_400;
    let h = (s % 86_400) / 3600;
    let m = (s % 3600) / 60;
    if d > 0 {
        format!("{d}d {h}h")
    } else if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m")
    }
}

/// "Just now" / "Xs ago" / "Xm ago" / "Xh ago" / RFC-3339-ish for older.
/// Falls back to the raw timestamp if the clock can't be read.
fn format_ago(unix_ts: u64) -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let Ok(now) = SystemTime::now().duration_since(UNIX_EPOCH) else {
        return unix_ts.to_string();
    };
    let now = now.as_secs();
    if unix_ts > now {
        return unix_ts.to_string();
    }
    let d = now - unix_ts;
    if d < 5 {
        "just now".into()
    } else if d < 60 {
        format!("{d}s ago")
    } else if d < 3600 {
        format!("{}m ago", d / 60)
    } else if d < 86_400 {
        format!("{}h ago", d / 3600)
    } else {
        format!("{}d ago", d / 86_400)
    }
}

/// Default: ERROR under `--daemonize`, else INFO. `-v` bumps, `-q` drops,
/// `RUST_LOG` overrides. Third-party crates are clamped at WARN so `-v` debug
/// doesn't drown the user in TLS-handshake chatter.
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
    b.filter_module("srunc", our_level);
    if let Ok(spec) = std::env::var("RUST_LOG") {
        b.parse_filters(&spec);
    }
    b.format(|buf, record| {
        let style = buf.default_level_style(record.level());
        let sym = match record.level() {
            log::Level::Error => "✗",
            log::Level::Warn => "⚠",
            log::Level::Info => "→",
            log::Level::Debug | log::Level::Trace => "·",
        };
        writeln!(buf, "{style}{sym}{style:#} {}", record.args())
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
        Cmd::Profiles { action } => match action {
            ProfilesCmd::List { json } => cmd_profiles_list(*json),
        },
    };

    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            print_error(&e, cli.verbose);
            ExitCode::FAILURE
        }
    }
}

/// User-facing rendering of the top-level error.
///
/// Default (no `-v`): walk the anyhow chain looking for the first typed
/// `srunc::Error` source, then print its short `user_message()` prefixed
/// by the anyhow context (e.g. "login failed: wrong password (E2553)").
/// If the chain has no typed leaf, fall back to the top-level Display.
///
/// With `-v`: emit the full chain (`{:#}`) so devs get all the context.
fn print_error(e: &anyhow::Error, verbose: u8) {
    use anstream::eprintln;
    use anstyle::{AnsiColor, Style};
    let red = Style::new().fg_color(Some(AnsiColor::Red.into())).bold();
    let dim = Style::new().dimmed();
    if verbose > 0 {
        eprintln!("{red}error:{red:#} {e:#}");
        return;
    }
    let typed = e
        .chain()
        .find_map(|c| c.downcast_ref::<srunc::Error>())
        .map(|s| s.user_message());
    let top = e.to_string();
    match typed {
        // anyhow's top-level message is the .context() string we attached
        // (e.g. "login failed"); show that as the headline and the typed
        // message as the explanation. If the top-level *is* the typed
        // error (no context wrapper), avoid duplication.
        Some(msg) if msg.as_ref() != top => {
            eprintln!("{red}error:{red:#} {top}: {msg}");
            eprintln!("       {dim}run with -v for full detail{dim:#}");
        }
        Some(msg) => {
            eprintln!("{red}error:{red:#} {msg}");
            eprintln!("       {dim}run with -v for full detail{dim:#}");
        }
        // No typed leaf — fall back to anyhow's full chain so generic
        // failures (config parse, missing field, …) still carry their
        // detail.
        None => eprintln!("{red}error:{red:#} {e:#}"),
    }
}
