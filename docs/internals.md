# srunc — internals

Engineering reference: architecture, build, release. Not needed to *use* srunc — see [`../README.md`](../README.md) for that.

## Layout

Single crate `srunc`. Binary at `src/main.rs` (requires the `cli` default feature); library API in `src/lib.rs` + submodules:

```
src/
├── lib.rs             re-exports
├── main.rs            CLI binary (requires `cli` feature)
├── protocol/          pure, side-effect-free protocol crypto
│   ├── xencode.rs     srun's XXTEA-variant block cipher
│   ├── base64.rs      QuirkBase64Encode (custom alphabet)
│   ├── digest.rs      MD5 / SHA1
│   ├── params.rs      build_portal_params (login/logout assembly)
│   └── error.rs       typed PortalError + code→message table
├── client.rs          ureq-based HTTP client
├── endpoints.rs       per-host URL set (generic over deployment)
├── discover.rs        captive-redirect parsing → {host, ac_id}, DNS-free
├── gateway.rs         default-gateway IP via /proc/net/route
├── models.rs          typed serde responses
├── jsonp.rs           strip `callback(...)` JSONP wrappers
└── profiles.rs        built-in {host, ac_id} for known srun deployments

tests/
├── protocol_vectors.rs    pinned byte-for-byte golden vectors for the protocol primitives
├── portal_mock.rs         httpmock-based Client tests
└── cli.rs                 end-to-end CLI tests
```

Library users opt out of the binary's deps with `cargo add srunc --no-default-features` — drops `clap`, `anstream`, `env_logger`, `rpassword`, `ctrlc`, `basic-toml`, `anyhow`.

## Why this shape

- **Crypto pinned by golden vectors.** `tests/protocol_vectors.rs` pins XEncode / QuirkBase64Encode / `build_portal_params` byte-for-byte, so any subtle change to encoding, padding, word order, or parameter assembly fails CI immediately.
- **No global mutable state.** Timeout and HTTP agent live on `Client`.
- **Typed portal errors.** `PortalError::from_code` returns `WrongPassword` / `AlreadyOnline` / … instead of string compares.
- **Sync I/O via `ureq` + `rustls`, no tokio** — one keep-alive timer doesn't justify an async runtime.
- **Zero deployment-specific code paths.** `discover` derives `(host, ac_id)` from the captive redirect; `profiles` is a tiny built-in lookup for the rare cases where discovery misses.

## Build / test

```sh
cargo fmt --all -- --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked --all-targets        # 49 tests (5 vectors + 27 unit + 7 + 10 integration)
cargo build --release --locked           # ~2.3 MB at target/release/srunc
cargo build --locked --no-default-features  # library-only (proves no CLI dep leakage)
```

## Cross-platform distribution

`release.yml` ships, per tagged release: 9 binary tarballs, 5 OpenWrt `.ipk`s, 2 `.deb`s, a `SHA256SUMS`, plus a `cargo publish` to crates.io.

| OS | Targets |
|----|---------|
| Linux glibc + static musl | `x86_64`, `aarch64` |
| Linux static musl (routers) | `armv7-musleabihf`, `arm-musleabi` |
| macOS | `x86_64`, `aarch64` |
| Windows | `x86_64-msvc` |

Linux cross targets built with [`cross`](https://github.com/cross-rs/cross). Musl targets get UPX (`--best --lzma`, ~60% smaller for routers). Size-tuned release profile: `opt-level="z"`, LTO, `panic="abort"`, `strip`.

### MIPS (best-effort)

`mipsel` / `mips` are Rust tier-3; need `-Z build-std` on nightly:

```sh
cargo +nightly build --release -Z build-std=std,panic_abort \
  --target mipsel-unknown-linux-musl
```

Most actively-supported OpenWrt devices in the past five years are armv7 / aarch64 (MT7621 / MT7986 / IPQ60xx) — the default matrix covers those.

## Adding a deployment profile

[`src/profiles.rs`](../src/profiles.rs) is a compile-time slice. Append a `Profile { id, label, host, ac_id }` (alphabetical by `id`), run `cargo test`, send a PR. Only add entries whose `host` / `ac_id` you have **actually verified on the network** — unverified guesses are worse than no entry, and propagate as a "looks like an srunc bug" report.

The OpenWrt LuCI dropdown picks new profiles up automatically via `ubus call srunc profiles` (no JS change needed).

A remote / network profile list was considered and rejected: profiles change rarely, and a network dependency for a settings-page dropdown adds meaningful supply-chain surface for nothing.
