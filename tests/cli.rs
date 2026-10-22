//! End-to-end tests for the `srunc` binary. Skipped under
//! `--no-default-features` since the binary isn't built without the `cli`
//! feature.

#![cfg(feature = "cli")]

use std::io::Write;

use assert_cmd::Command;
use httpmock::prelude::*;
use predicates::prelude::*;
use tempfile::NamedTempFile;

/// Build a fresh `Command` for the `srunc` binary, pre-configured so it
/// can't accidentally read the developer's real config from $HOME.
fn srunc() -> Command {
    let mut cmd = Command::cargo_bin("srunc").unwrap();
    cmd.env_remove("SRUNC_CONFIG")
        .env_remove("XDG_CONFIG_HOME")
        .env("HOME", "/nonexistent");
    cmd
}

fn write_temp_config(toml: &str) -> NamedTempFile {
    let mut f = NamedTempFile::new().unwrap();
    f.write_all(toml.as_bytes()).unwrap();
    f
}

// ---------------- smoke ----------------

#[test]
fn help_exits_zero_and_lists_subcommands() {
    srunc()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("login"))
        .stdout(predicate::str::contains("logout"))
        .stdout(predicate::str::contains("status"))
        .stdout(predicate::str::contains("profiles"))
        .stdout(predicate::str::contains("discover"));
}

#[test]
fn version_reports_package_version() {
    srunc()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

// ---------------- config loading ----------------

#[test]
fn missing_creds_in_daemonize_mode_errors_cleanly() {
    // No config, no -u/-p, -D forbids stdin prompts → must fail with a clear
    // message instead of hanging or panicking.
    let cfg = write_temp_config(""); // empty config: zero fields set
    srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "-D", "login"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("username required"));
}

#[test]
fn malformed_toml_surfaces_a_parse_error() {
    let cfg = write_temp_config("this is not = valid = toml at all\n[[[\n");
    srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("parsing config file"));
}

#[test]
fn unknown_field_in_config_is_rejected() {
    // ConfigFile uses `#[serde(deny_unknown_fields)]` — typos shouldn't be
    // silently ignored.
    let cfg =
        write_temp_config("username = \"alice\"\npassword = \"x\"\nunknown_field = \"oops\"\n");
    srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "-D", "status"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unknown"));
}

// ---------------- probe-ip override ----------------

fn mock_gateway_redirect(server: &MockServer, href: &str) {
    server.mock(|when, then| {
        when.method(GET).path("/");
        then.status(200).body(format!(
            r#"<html>
<head>
<script type="text/javascript">location.href="{href}"</script>
</head>
<body>
</body>
</html>"#
        ));
    });
}

#[test]
fn discover_json_uses_probe_ip_from_config() {
    let server = MockServer::start();
    let probe_ip = server.address().to_string();
    mock_gateway_redirect(
        &server,
        "http://10.42.0.16/index_169.html?wlanuserip=10.42.42.16&ssid=THUNET",
    );
    let cfg = write_temp_config(&format!("probe_ip = \"{probe_ip}\"\n"));

    let out = srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "--json", "discover"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(out.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout was: {stdout:?}"));
    assert_eq!(parsed["probe_ip"], probe_ip);
    assert_eq!(parsed["host"], "10.42.0.16");
    assert_eq!(parsed["ac_id"], "169");
}

#[test]
fn cli_probe_ip_overrides_config_and_default_route() {
    let server = MockServer::start();
    let probe_ip = server.address().to_string();
    mock_gateway_redirect(
        &server,
        "http://10.42.0.16/index_16.html?wlanuserip=10.42.42.16&ssid=THUNET",
    );

    let cfg = write_temp_config("probe_ip = \"192.0.2.1\"\n");

    let out = srunc()
        .args([
            "-c",
            cfg.path().to_str().unwrap(),
            "--probe-ip",
            &probe_ip,
            "--timeout",
            "1",
            "--json",
            "discover",
        ])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(out.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout was: {stdout:?}"));
    assert_eq!(parsed["probe_ip"], probe_ip);
    assert_eq!(parsed["host"], "10.42.0.16");
    assert_eq!(parsed["ac_id"], "16");
}

// ---------------- end-to-end against a mock portal ----------------

const CB: &str = "C_a_l_l_b_a_c_k";

fn mock_online_portal(server: &MockServer, username: &str, ip: &str) {
    server.mock(|when, then| {
        when.method(GET).path("/srun_portal_pc");
        then.status(200)
            .body(format!(r#"<html>online, ip : "{ip}"</html>"#));
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/rad_user_info");
        then.status(200).body(format!(
            r#"{CB}({{"error":"ok","user_name":"{username}"}})"#
        ));
    });
}

#[test]
fn status_json_emits_expected_shape() {
    let server = MockServer::start();
    mock_online_portal(&server, "alice", "10.0.0.42");

    let host = server.address().to_string(); // 127.0.0.1:<port>
    let cfg = write_temp_config(&format!(
        "username = \"alice\"\npassword = \"x\"\nhost = \"{host}\"\nac_id = \"1\"\ninsecure = true\n"
    ));

    let out = srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "-D", "--json", "status"])
        .assert()
        .success()
        .get_output()
        .clone();

    let stdout = String::from_utf8(out.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout was: {stdout:?}"));
    assert_eq!(parsed["online"], serde_json::Value::Bool(true));
    assert_eq!(parsed["username"], "alice");
    assert_eq!(parsed["ip"], "10.0.0.42");
}

#[test]
fn status_human_output_for_online() {
    let server = MockServer::start();
    mock_online_portal(&server, "bob", "10.0.0.7");

    let host = server.address().to_string();
    let cfg = write_temp_config(&format!(
        "username = \"bob\"\npassword = \"x\"\nhost = \"{host}\"\nac_id = \"1\"\ninsecure = true\n"
    ));

    srunc()
        .args(["-c", cfg.path().to_str().unwrap(), "-D", "status"])
        .assert()
        .success()
        // anstream strips ANSI colors automatically when stdout isn't a TTY,
        // so assert_cmd's captured output is plain text.
        .stdout(predicate::str::contains("● Online"))
        .stdout(predicate::str::contains("bob"))
        .stdout(predicate::str::contains("10.0.0.7"));
}

#[test]
fn profiles_list_json_returns_at_least_tsinghua() {
    let out = srunc()
        .args(["profiles", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .clone();
    let stdout = String::from_utf8(out.stdout).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|e| panic!("not valid JSON: {e}\nstdout was: {stdout:?}"));
    let arr = parsed.as_array().expect("profiles list --json is an array");
    assert!(!arr.is_empty(), "registry must ship at least one profile");
    let tsinghua = arr
        .iter()
        .find(|p| p["id"] == "tsinghua")
        .expect("tsinghua profile should ship by default");
    assert_eq!(tsinghua["host"], "auth4.tsinghua.edu.cn");
    assert!(tsinghua["ac_id"].is_null());
}

#[test]
fn unknown_profile_warns_but_doesnt_crash() {
    // --profile typo: should log a warning and fall through to the rest of
    // the resolution chain rather than panicking or hard-erroring.
    let cfg = write_temp_config("");
    srunc()
        .args([
            "-c",
            cfg.path().to_str().unwrap(),
            "-D",
            "--profile",
            "not-a-real-site",
            "login",
        ])
        .assert()
        .failure() // still fails on missing creds, not on the bad profile
        .stderr(predicate::str::contains("username required"));
}

#[test]
fn quiet_flag_suppresses_info_logs_on_stderr() {
    let server = MockServer::start();
    mock_online_portal(&server, "alice", "10.0.0.42");

    let host = server.address().to_string();
    let cfg = write_temp_config(&format!(
        "username = \"alice\"\npassword = \"x\"\nhost = \"{host}\"\nac_id = \"1\"\ninsecure = true\n"
    ));

    // -qq drops the level below INFO; no "INFO" lines should appear on stderr.
    srunc()
        .args([
            "-c",
            cfg.path().to_str().unwrap(),
            "-qq",
            "--json",
            "status",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("INFO").not());
}
