//! Thin imperative shell over `srun-core`.
//!
//! This intentionally does NOT speak HTTP yet — it demonstrates the architecture
//! and gives a distributable artifact that exercises the pure core:
//!   * `params`   — compute the portal login/logout parameters offline
//!   * `selftest` — verify the crypto core against the Go reference vectors
//!
//! The networking layer (get_challenge -> login -> keep-alive) is the next step
//! and belongs in its own `srun` I/O crate; see the project README.

use clap::{Parser, Subcommand};
use srun_core::{build_portal_params, quirk_base64, sha1_hex, xencode, Action, Credentials};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "auth-thu-rs", version, about = "srun portal auth — Rust core (offline tooling + self-test)")]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Compute portal login/logout parameters offline (no network needed).
    Params {
        #[arg(short, long)]
        username: String,
        #[arg(short, long, default_value = "")]
        password: String,
        /// The challenge token returned by /cgi-bin/get_challenge.
        #[arg(short, long)]
        token: String,
        #[arg(long, default_value = "")]
        ip: String,
        #[arg(long, default_value = "1")]
        ac_id: String,
        #[arg(long)]
        logout: bool,
    },
    /// Verify the crypto core against the Go reference vectors.
    Selftest,
}

fn main() -> ExitCode {
    match Cli::parse().cmd {
        Cmd::Params { username, password, token, ip, ac_id, logout } => {
            let creds = Credentials { username: &username, password: &password };
            let action = if logout { Action::Logout } else { Action::Login };
            let p = build_portal_params(&creds, &token, action, &ip, &ac_id);
            println!("action       = {}", p.action);
            println!("username     = {}", p.username);
            if let Some(pw) = &p.password {
                println!("password     = {pw}");
            }
            println!("ac_id        = {}", p.ac_id);
            println!("ip           = {}", p.ip);
            println!("n            = {}", p.n);
            println!("type         = {}", p.type_);
            println!("double_stack = {}", p.double_stack);
            println!("info         = {}", p.info);
            println!("chksum       = {}", p.chksum);
            ExitCode::SUCCESS
        }
        Cmd::Selftest => {
            if run_selftest() {
                println!("selftest: OK");
                ExitCode::SUCCESS
            } else {
                eprintln!("selftest: FAILED");
                ExitCode::FAILURE
            }
        }
    }
}

fn run_selftest() -> bool {
    let mut ok = true;
    let mut check = |name: &str, got: String, want: &str| {
        if got != want {
            ok = false;
            eprintln!("  {name}: got {got:?}, want {want:?}");
        }
    };

    let tok = "aa0edd0fff7dd9f1f0ae4e981ec0114c7b0bf6f67c4895bed4f4ac634e97ecf2";
    check("quirk_base64", quirk_base64(b"567"), "0FZ7");
    check("xencode", quirk_base64(&xencode(b"1", tok.as_bytes())), "NmsaR0fCm5H=");
    check("sha1", sha1_hex(b"abc"), "a9993e364706816aba3e25717850c26c9cd0d89d");

    let p = build_portal_params(
        &Credentials { username: "hello", password: "pass" },
        "32f23b9c2229fd034f6d5160d8b4536496af550efc45113635689d2d8f12ffad",
        Action::Login,
        "",
        "1",
    );
    check("chksum", p.chksum, "8fd747a5b37c63f695b23fa2bd0339b1a961aa49");

    ok
}
