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
                  the rest. On platforms without /proc (macOS / Windows), \
                  pass `--probe-ip <ip>` so discovery can find the portal.",
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

    /// IPv4 address to probe for the captive-portal redirect. Defaults to the
    /// system default gateway (read from /proc/net/route on Linux); set this
    /// on platforms without /proc (macOS / Windows) or to probe a different
    /// address (AP / portal controller).
    #[arg(long, global = true, value_name = "IP")]
    probe_ip: Option<String>,

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
    Status {
        /// Re-render the status block every `--interval` seconds until
        /// Ctrl-C. Useful as a companion to `login --keepalive`.
        #[arg(short, long)]
        watch: bool,
        /// Seconds between refreshes when `--watch` is set.
        #[arg(long, default_value_t = 5, value_name = "SECS")]
        interval: u64,
    },
    /// Manage deployment profiles (built-in host + ac_id for known srun installs).
    Profiles {
        #[command(subcommand)]
        action: ProfilesCmd,
    },
    /// Probe the default gateway (or `--probe-ip`) for the captive-portal
    /// redirect and print the host + ac_id it advertises. Backs the
    /// "Discover from gateway" button in the LuCI Settings view.
    Discover,
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
    probe_ip: Option<String>,
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
    probe_ip: Option<String>,
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
        probe_ip: cli.probe_ip.clone().or(cfg.probe_ip),
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
    /// `true` when `ac_id` is the [`STATUS_FALLBACK_AC_ID`] placeholder rather
    /// than a genuinely resolved value (only possible on the `status` path,
    /// where ac_id isn't required). Callers that surface the ac_id hide it.
    ac_id_synthetic: bool,
}

impl Resolved {
    /// The ac_id to surface to UIs: `None` when it's only the placeholder, so
    /// the dashboard doesn't display a fake segment id.
    fn display_ac_id(&self) -> Option<String> {
        (!self.ac_id_synthetic).then(|| self.ac_id.clone())
    }
}

/// `status`'s online check is keyed by the source IP the portal sees, so its
/// `ac_id` is just a required query parameter, never a discriminator. When we
/// can't resolve a real one (an ac_id-less profile with no login cache yet),
/// this benign placeholder lets the check proceed instead of failing outright.
const STATUS_FALLBACK_AC_ID: &str = "1";

/// Address to probe for captive-portal discovery. An explicit `--probe-ip` /
/// config `probe_ip` always wins; otherwise fall back to the platform default
/// gateway (Linux `/proc/net/route`; `None` on platforms without /proc).
fn probe_target(s: &Settings) -> Option<String> {
    s.probe_ip
        .clone()
        .or_else(srunc::gateway::default_gateway_v4)
}

/// Fallback chain: explicit `--host`/`--ac-id` → `--profile` preset →
/// captive-portal discovery (probe default gateway). `require_ac_id` is set by
/// the auth paths (login/logout) that genuinely need a segment-correct ac_id;
/// `status` clears it, because once we're authenticated the captive redirect
/// that discovery relies on is gone — re-probing for an ac_id we don't even
/// use would make `status` fail at the exact moment it should report "online".
/// Errors out if a *required* value can't be found, rather than silently
/// picking a stale default.
fn resolve_target(s: &Settings, require_ac_id: bool) -> Result<Resolved> {
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

    let host_known = s.host.is_some() || profile_host.is_some();
    let ac_id_known = s.ac_id.is_some() || profile_ac_id.is_some();
    // Only probe for something we'll actually use: `status` needs just the
    // host, so it never triggers discovery for a missing ac_id.
    let need_discovery = !host_known || (require_ac_id && !ac_id_known);
    let discovered = if need_discovery {
        probe_target(s)
            .and_then(|probe_ip| {
                info!("probing {probe_ip} for srun portal hints");
                match srunc::discover(&probe_ip, s.timeout) {
                    Ok(d) => Some(d),
                    Err(e) => {
                        debug!("portal probe failed: {e}");
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
                "could not determine portal host — pass --host or --profile <id>, \
                 or pass --probe-ip <ip> (or ensure the default route redirects \
                 to a captive portal)"
            )
        })?;
    let endpoints = Endpoints::from_host(&host, s.insecure);

    let resolved_ac_id = s.ac_id.clone().or(profile_ac_id).or(discovered.ac_id);
    let (ac_id, ac_id_synthetic) = match resolved_ac_id {
        Some(v) => (v, false),
        // status path: proceed with a placeholder (the online check ignores it).
        None if !require_ac_id => (STATUS_FALLBACK_AC_ID.to_string(), true),
        None => bail!(
            "could not determine ac_id — pass --ac-id or --profile <id>, \
             or pass --probe-ip <ip> (or ensure the captive redirect carries \
             an index_<N>.html / ac_id=<N> hint)"
        ),
    };

    info!("resolved: host={host} ac_id={ac_id}");
    Ok(Resolved {
        host,
        endpoints,
        ac_id,
        ac_id_synthetic,
    })
}

/// Path where `cmd_login` caches the resolved target for `cmd_status` to reuse,
/// a sibling of the config file: `…/<name>.toml` → `…/<name>.resolved`.
fn resolved_cache_path(cfg: &Path) -> PathBuf {
    cfg.with_extension("resolved")
}

/// Best-effort: record the host + ac_id resolved at login next to the config,
/// so a later `status` — which can no longer re-discover them once we're
/// online — can reuse them and query the portal directly. Failures are
/// swallowed: the cache is an optimization, not a correctness requirement.
fn write_resolved_cache(cfg: &Path, host: &str, ac_id: &str) {
    let path = resolved_cache_path(cfg);
    // `{:?}` quotes/escapes like a TOML basic string; host + ac_id never carry
    // exotic characters, so this round-trips through `ConfigFile`.
    let body = format!("host = {host:?}\nac_id = {ac_id:?}\n");
    if fs::write(&path, body).is_err() {
        return;
    }
    // Match the rendered config's 0600 — the cache lives beside it in tmpfs.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
}

/// Read back a cache written by [`write_resolved_cache`]. Returns `Some` only
/// when the file parses and both fields are present.
fn read_resolved_cache(cfg: &Path) -> Option<(String, String)> {
    let text = fs::read_to_string(resolved_cache_path(cfg)).ok()?;
    let parsed: ConfigFile = basic_toml::from_str(&text).ok()?;
    Some((parsed.host?, parsed.ac_id?))
}

/// Fold the login-time resolved cache (host + ac_id) into `s` when present —
/// lower priority than explicit cli/config values, higher than profile or
/// discovery. This is what lets `status` and `logout` run *while online*: by
/// then the captive redirect that discovery needs is gone, so the cached pair
/// is the only way to reach the portal. No-op without `--config` or a cache.
fn apply_resolved_cache(cli: &Cli, s: &mut Settings) {
    let Some(cfg) = cli.config.as_deref() else {
        return;
    };
    let Some((host, ac_id)) = read_resolved_cache(cfg) else {
        return;
    };
    s.host = s.host.take().or(Some(host));
    s.ac_id = s.ac_id.take().or(Some(ac_id));
}

fn cmd_discover(cli: &Cli) -> Result<()> {
    // Settings resolution gives us the timeout the user has configured —
    // discovery shouldn't hang the LuCI page if the probe target is unreachable.
    let s = resolve(cli)?;
    let probe_ip = probe_target(&s).ok_or_else(|| {
        anyhow!(
            "no probe target — pass --probe-ip <ip> or set `probe_ip` in the \
             config (Linux reads the default gateway from /proc/net/route)"
        )
    })?;
    let d = srunc::discover(&probe_ip, s.timeout).context("portal probe failed")?;

    if cli.json {
        // Emit the same shape regardless of partial results so JS callers
        // (the LuCI Discover button) can always read both keys.
        let payload = serde_json::json!({
            "probe_ip": probe_ip,
            "host": d.host,
            "ac_id": d.ac_id,
        });
        println!("{}", serde_json::to_string(&payload)?);
        return Ok(());
    }

    use anstream::println;
    use anstyle::Style;
    let dim = Style::new().dimmed();
    let bold = Style::new().bold();
    println!("{dim}probe_ip{dim:#}  {bold}{}{bold:#}", probe_ip);
    println!(
        "{dim}host{dim:#}      {bold}{}{bold:#}",
        d.host.as_deref().unwrap_or("(not found)")
    );
    println!(
        "{dim}ac_id{dim:#}    {bold}{}{bold:#}",
        d.ac_id.as_deref().unwrap_or("(not found)")
    );
    Ok(())
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
        println!("{glyph_style}{glyph}{glyph_style:#} {bold}{}{bold:#}", p.id);
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

    // Discovery-first so a normal (offline) login always picks the
    // segment-fresh ac_id. Only when that fails — which is precisely the
    // "re-login while still online" case (procd respawning us after a keepalive
    // blip, with the portal session still valid) — fall back to the target
    // cached at the previous successful login. The fallback never fires on a
    // genuine offline login, so a roaming client still re-discovers fresh.
    let r = match resolve_target(&s, true) {
        Ok(r) => r,
        Err(e) => {
            apply_resolved_cache(cli, &mut s);
            resolve_target(&s, true).map_err(|_| e)?
        }
    };
    // Record the resolved target so `status` / `logout` can reuse it once we're
    // online (the captive redirect is gone after auth). Best-effort.
    if let Some(cfg) = cli.config.as_deref() {
        write_resolved_cache(cfg, &r.host, &r.ac_id);
    }
    let client = Client::new(r.endpoints, s.timeout);

    // Idempotent: if the portal already considers us online (the session
    // outlived the keepalive blip that made procd respawn us), don't re-submit
    // credentials — some deployments reject a duplicate login, which would turn
    // a recoverable blip into a respawn storm. Resume keepalive directly. A
    // failed status probe falls through to a normal login attempt.
    if client.status(&r.ac_id).is_ok_and(|st| st.online) {
        info!("already online; skipping login");
    } else {
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
    // De-auth happens while we're online, when the captive redirect discovery
    // needs is gone — reuse the ac_id the daemon cached at login so the portal
    // logout targets the right segment instead of failing to resolve.
    apply_resolved_cache(cli, &mut s);
    let r = resolve_target(&s, true)?;
    let client = Client::new(r.endpoints, s.timeout);
    client
        .logout(&s.username, &r.ac_id, &s.ip)
        .context("logout failed")?;
    info!("logout: ok");
    Ok(())
}

fn cmd_status(cli: &Cli, watch: bool, interval: u64) -> Result<()> {
    let mut s = resolve(cli)?;
    // Reuse the target the daemon cached at login so `status` keeps reporting
    // "online" and talks to the portal directly instead of re-probing the
    // gateway (which can't work once we're authenticated).
    apply_resolved_cache(cli, &mut s);
    let r = resolve_target(&s, false)?;
    // Snapshot the display values before `r.endpoints` is moved into the client.
    let resolved_host = Some(r.host.clone());
    let resolved_ac_id = r.display_ac_id();
    let client = Client::new(r.endpoints, s.timeout);

    if !watch {
        let mut st = client.status(&r.ac_id).context("status query failed")?;
        st.resolved_host = resolved_host;
        st.resolved_ac_id = resolved_ac_id;
        emit_status(&st, cli.json);
        return Ok(());
    }

    // --watch: refresh until Ctrl-C. JSON mode emits NDJSON (one object per
    // line) instead of clear-and-redraw — friendlier to pipes / tee. Human
    // mode clears the screen and re-renders in place.
    let interval = interval.max(1);
    let stop = install_signal_handler()?;
    let mut backoff = interval;
    while !stop.load(Ordering::SeqCst) {
        match client.status(&r.ac_id) {
            Ok(mut st) => {
                st.resolved_host = resolved_host.clone();
                st.resolved_ac_id = resolved_ac_id.clone();
                if !cli.json {
                    anstream::print!("\x1b[2J\x1b[H");
                }
                emit_status(&st, cli.json);
                backoff = interval;
            }
            Err(e) => {
                // Log but don't bail — transient network hiccups shouldn't
                // kill the watch loop. Back off so a portal outage doesn't
                // spam the screen; reset on recovery.
                warn!("status: {}: {}", e, srunc::Error::user_message(&e));
                backoff = (backoff.saturating_mul(2)).min(60);
            }
        }
        sleep_interruptible(backoff, &stop);
    }
    Ok(())
}

fn emit_status(st: &srunc::Status, json: bool) {
    if json {
        // Already on its own line; under --watch this gives NDJSON.
        println!("{}", serde_json::to_string(st).unwrap_or_default());
    } else {
        print_status_human(st);
    }
}

/// Sleep for `secs` 1-second slices, returning early when `stop` flips.
fn sleep_interruptible(secs: u64, stop: &AtomicBool) {
    let mut left = secs;
    while left > 0 && !stop.load(Ordering::SeqCst) {
        std::thread::sleep(Duration::from_secs(1));
        left -= 1;
    }
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
        Cmd::Status { watch, interval } => cmd_status(&cli, *watch, *interval),
        Cmd::Profiles { action } => match action {
            ProfilesCmd::List { json } => cmd_profiles_list(*json),
        },
        Cmd::Discover => cmd_discover(&cli),
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

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(host: Option<&str>, ac_id: Option<&str>, profile: Option<&str>) -> Settings {
        Settings {
            username: "u".into(),
            password: "p".into(),
            probe_ip: None,
            host: host.map(str::to_string),
            insecure: false,
            ac_id: ac_id.map(str::to_string),
            ip: String::new(),
            timeout: Duration::from_secs(2),
            daemonize: false,
            profile: profile.map(str::to_string),
        }
    }

    #[test]
    fn explicit_probe_ip_wins_over_default_route() {
        // `--probe-ip` / config `probe_ip` is what makes discovery work on
        // platforms with no /proc: it must be used even when the host can also
        // read a default route (Linux).
        let mut s = settings(None, None, None);
        s.probe_ip = Some("192.0.2.1".into());
        assert_eq!(probe_target(&s).as_deref(), Some("192.0.2.1"));
    }

    #[test]
    fn status_path_tolerates_missing_ac_id() {
        // Tsinghua pins host only. With require_ac_id=false the host resolves
        // from the profile, ac_id falls back to the placeholder, and — because
        // the host is already known — no gateway probe is attempted (so this
        // test stays hermetic / network-free).
        let s = settings(None, None, Some("tsinghua"));
        let r = resolve_target(&s, false).expect("status must not require an ac_id");
        assert_eq!(r.host, "auth4.tsinghua.edu.cn");
        assert_eq!(r.ac_id, STATUS_FALLBACK_AC_ID);
        assert!(r.ac_id_synthetic);
        assert_eq!(r.display_ac_id(), None);
    }

    #[test]
    fn pinned_profile_resolves_both_without_discovery() {
        // zhejiang pins host + ac_id, so even the strict (login) path needs no
        // gateway probe.
        let s = settings(None, None, Some("zhejiang"));
        let r = resolve_target(&s, true).expect("fully-pinned profile resolves");
        assert_eq!(r.host, "net.zju.edu.cn");
        assert_eq!(r.ac_id, "3");
        assert!(!r.ac_id_synthetic);
        assert_eq!(r.display_ac_id().as_deref(), Some("3"));
    }

    #[test]
    fn explicit_ac_id_is_surfaced_not_synthetic() {
        let s = settings(Some("portal.example.com"), Some("7"), None);
        let r = resolve_target(&s, false).expect("explicit host+ac_id resolves");
        assert_eq!(r.host, "portal.example.com");
        assert_eq!(r.ac_id, "7");
        assert!(!r.ac_id_synthetic);
        assert_eq!(r.display_ac_id().as_deref(), Some("7"));
    }

    #[test]
    fn resolved_cache_round_trips() {
        let mut cfg = std::env::temp_dir();
        cfg.push(format!("srunc-cache-test-{}.toml", std::process::id()));
        write_resolved_cache(&cfg, "auth4.tsinghua.edu.cn", "169");
        let got = read_resolved_cache(&cfg);
        let _ = fs::remove_file(resolved_cache_path(&cfg));
        assert_eq!(
            got,
            Some(("auth4.tsinghua.edu.cn".to_string(), "169".to_string()))
        );
    }

    #[test]
    fn missing_cache_reads_none() {
        let cfg = std::env::temp_dir().join("srunc-cache-absent-xyzzy.toml");
        let _ = fs::remove_file(resolved_cache_path(&cfg));
        assert_eq!(read_resolved_cache(&cfg), None);
    }
}
