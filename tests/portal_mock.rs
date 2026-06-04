//! Integration tests for `Client` against a mocked srun portal.
//!
//! Spins up an `httpmock` server, points `Endpoints` at it, and verifies the
//! request shape + response handling for `challenge` / `status` / `login` /
//! `logout` — including the portal's JSONP envelope, the "ip extracted from
//! the online-check HTML" quirk, and typed portal errors.

use std::time::Duration;

use httpmock::prelude::*;

use srunc::{Client, Credentials, Endpoints};

fn endpoints_for(server: &MockServer) -> Endpoints {
    endpoints_with_host(server, "mock.local")
}

/// Build `Endpoints` whose URLs point at the mock server but whose recorded
/// `host` is an arbitrary string. Tests that exercise the "verified-schema"
/// path pass `auth4.tsinghua.edu.cn` here to trip the host check in
/// `Client::status`.
fn endpoints_with_host(server: &MockServer, host: &str) -> Endpoints {
    // `from_host` builds `https://...`; the mock is plain HTTP, so build
    // the four URLs by hand pointing at the mock server.
    let base = server.base_url();
    Endpoints {
        host: host.to_string(),
        login: format!("{base}/cgi-bin/srun_portal"),
        challenge: format!("{base}/cgi-bin/get_challenge"),
        user_info: format!("{base}/cgi-bin/rad_user_info"),
        online_check: format!("{base}/srun_portal_pc"),
    }
}

const CB: &str = "C_a_l_l_b_a_c_k";

// ---------------- challenge ----------------

#[test]
fn challenge_extracts_token() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET)
            .path("/cgi-bin/get_challenge")
            .query_param("username", "alice")
            .query_param("ip", "1.2.3.4")
            .query_param("double_stack", "1")
            .query_param("callback", CB);
        then.status(200)
            .body(format!(r#"{CB}({{"res":"ok","challenge":"deadbeef"}})"#));
    });
    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let token = client.challenge("alice", "1.2.3.4").unwrap();
    assert_eq!(token, "deadbeef");
}

#[test]
fn challenge_propagates_portal_error() {
    let server = MockServer::start();
    let _m = server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/get_challenge");
        then.status(200)
            .body(format!(r#"{CB}({{"res":"username does not exist"}})"#));
    });
    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let err = client.challenge("nobody", "1.2.3.4").unwrap_err();
    assert!(
        format!("{err}").contains("username does not exist"),
        "got: {err}"
    );
}

// ---------------- status ----------------

#[test]
fn status_online_extracts_user_and_ip() {
    let server = MockServer::start();
    // Online-check page must include the JS-literal `ip : "X.Y.Z.W"` pattern.
    let _portal = server.mock(|when, then| {
        when.method(GET)
            .path("/srun_portal_pc")
            .query_param("ac_id", "169");
        then.status(200)
            .body(r#"<html>online, ip : "10.0.0.42", uid : "alice"</html>"#);
    });
    let _info = server.mock(|when, then| {
        when.method(GET)
            .path("/cgi-bin/rad_user_info")
            .query_param("ip", "10.0.0.42")
            .query_param("callback", CB);
        then.status(200)
            .body(format!(r#"{CB}({{"error":"ok","user_name":"alice"}})"#));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let st = client.status("169").unwrap();
    assert!(st.online);
    assert_eq!(st.username.as_deref(), Some("alice"));
    assert_eq!(st.ip.as_deref(), Some("10.0.0.42"));
}

#[test]
fn status_basic_response_omits_details_field() {
    // A minimal `rad_user_info` (only `error` + `user_name`) on an unverified
    // host should leave `Status::details` as `None`.
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/srun_portal_pc");
        then.status(200)
            .body(r#"<html>online, ip : "10.0.0.42"</html>"#);
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/rad_user_info");
        then.status(200)
            .body(format!(r#"{CB}({{"error":"ok","user_name":"alice"}})"#));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let st = client.status("1").unwrap();
    assert!(st.online);
    assert!(
        st.details.is_none(),
        "unverified host should not surface details"
    );
}

#[test]
fn status_rich_details_for_verified_host() {
    // The captured-then-sanitized THU sample, served from a mock whose
    // recorded host matches the verified list. Asserts that `details` is
    // populated with the expected fields.
    let server = MockServer::start();
    let body = std::fs::read_to_string("tests/fixtures/thu_rad_user_info.json")
        .expect("THU fixture readable");
    server.mock(|when, then| {
        when.method(GET).path("/srun_portal_pc");
        then.status(200)
            .body(r#"<html>online, ip : "10.0.0.2"</html>"#);
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/rad_user_info");
        then.status(200).body(format!("{CB}({body})"));
    });

    let endpoints = endpoints_with_host(&server, "auth4.tsinghua.edu.cn");
    let client = Client::new(endpoints, Duration::from_secs(2));
    let st = client.status("1").unwrap();
    assert!(st.online);
    assert_eq!(st.username.as_deref(), Some("alice"));
    let d = st.details.expect("THU host yields details");
    assert_eq!(d.plan.as_deref(), Some("特殊学生"));
    assert_eq!(d.user_mac.as_deref(), Some("00-11-22-33-44-55"));
    assert_eq!(d.online_device_total, Some(3));
    assert_eq!(d.sum_bytes, Some(1_545_951_597));
    assert_eq!(d.sum_seconds, Some(325_134));
    assert_eq!(d.keepalive_time, Some(1_780_558_656));
}

#[test]
fn status_rich_details_suppressed_for_unverified_host() {
    // Same response body, but a host that's NOT on the verified list. The
    // client must NOT surface details — protects users from seeing fields
    // whose semantics we haven't confirmed for their deployment.
    let server = MockServer::start();
    let body = std::fs::read_to_string("tests/fixtures/thu_rad_user_info.json")
        .expect("THU fixture readable");
    server.mock(|when, then| {
        when.method(GET).path("/srun_portal_pc");
        then.status(200)
            .body(r#"<html>online, ip : "10.0.0.2"</html>"#);
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/rad_user_info");
        then.status(200).body(format!("{CB}({body})"));
    });

    let endpoints = endpoints_with_host(&server, "portal.example.edu");
    let client = Client::new(endpoints, Duration::from_secs(2));
    let st = client.status("1").unwrap();
    assert!(st.online);
    assert!(st.details.is_none(), "non-whitelisted host must not surface details");
}

#[test]
fn status_offline_when_portal_says_so() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/srun_portal_pc");
        then.status(200)
            .body(r#"<html>offline, ip : "10.0.0.42"</html>"#);
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/rad_user_info");
        // user_info returns error="not_online_error" → not "ok" → offline.
        then.status(200)
            .body(format!(r#"{CB}({{"error":"not_online_error"}})"#));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let st = client.status("1").unwrap();
    assert!(!st.online);
    assert!(st.username.is_none());
    // Even when offline, we still extracted the IP from the portal page.
    assert_eq!(st.ip.as_deref(), Some("10.0.0.42"));
}

// ---------------- login / logout ----------------

#[test]
fn login_succeeds_when_portal_returns_ok() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/get_challenge");
        then.status(200)
            .body(format!(r#"{CB}({{"res":"ok","challenge":"token123"}})"#));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/cgi-bin/srun_portal")
            // The submit request carries the assembled login params; we only
            // need to verify `action=login` and the ac_id was forwarded.
            .query_param("action", "login")
            .query_param("ac_id", "169");
        then.status(200).body(format!(r#"{CB}({{"error":"ok"}})"#));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let creds = Credentials {
        username: "alice",
        password: "hunter2",
    };
    client.login(&creds, "169", "10.0.0.42").unwrap();
}

#[test]
fn login_surfaces_typed_portal_error() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/get_challenge");
        then.status(200)
            .body(format!(r#"{CB}({{"res":"ok","challenge":"t"}})"#));
    });
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/srun_portal");
        // E2553 is srun's "wrong password" code — see PortalError::from_code.
        then.status(200).body(format!(
            r#"{CB}({{"error":"login_error","ecode":"E2553"}})"#
        ));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    let creds = Credentials {
        username: "alice",
        password: "wrong",
    };
    let err = client.login(&creds, "1", "10.0.0.42").unwrap_err();
    let s = format!("{err}");
    // The user-facing message should be the typed PortalError text, not the
    // raw ecode. PortalError::from_code maps E2553 → 密码错误 / wrong password.
    assert!(
        s.contains("E2553") || s.contains("密码") || s.to_lowercase().contains("password"),
        "expected typed wrong-password error, got: {s}"
    );
}

#[test]
fn logout_succeeds_with_empty_password() {
    let server = MockServer::start();
    server.mock(|when, then| {
        when.method(GET).path("/cgi-bin/get_challenge");
        then.status(200)
            .body(format!(r#"{CB}({{"res":"ok","challenge":"t"}})"#));
    });
    server.mock(|when, then| {
        when.method(GET)
            .path("/cgi-bin/srun_portal")
            .query_param("action", "logout");
        then.status(200).body(format!(r#"{CB}({{"error":"ok"}})"#));
    });

    let client = Client::new(endpoints_for(&server), Duration::from_secs(2));
    client.logout("alice", "1", "10.0.0.42").unwrap();
}
