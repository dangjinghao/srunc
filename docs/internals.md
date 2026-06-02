# srunc — internals

Engineering reference: architecture, build, cross-compile, release. Not needed to *use* srunc — see [`../README.md`](../README.md) for that.

## Architecture

**Functional core, imperative shell** — the hard, error-prone reverse-engineered crypto is a pure, side-effect-free crate; HTTP and HTML scraping live in a thin replaceable I/O layer.

```
crates/
├── srun-core/        PURE, no_std, zero I/O, zero globals       (crates.io: srun-core)
│   ├── src/xencode.rs    XEncode — srun's XXTEA-variant block cipher
│   ├── src/base64.rs     QuirkBase64Encode (custom alphabet)
│   ├── src/digest.rs     MD5 / SHA1 (RustCrypto)
│   ├── src/params.rs     build_portal_params (login/logout assembly)
│   ├── src/error.rs      typed PortalError + code→message table
│   └── tests/vectors.rs  golden vectors pinned to the Go reference
│
├── srun/             I/O shell (std)                            (crates.io: srun-client)
│   ├── src/client.rs     ureq-based HTTP client speaking the srun protocol
│   ├── src/endpoints.rs  per-host URL set (generic over deployment)
│   ├── src/discover.rs   captive-redirect parsing → {host, ac_id}, DNS-free
│   ├── src/gateway.rs    default-gateway discovery via /proc/net/route
│   ├── src/models.rs     typed serde responses
│   ├── src/jsonp.rs      strip `callback(...)` JSONP wrappers
│   └── src/profiles.rs   built-in {host, ac_id} for known srun deployments (start: tsinghua)
│
└── srun-cli/         this binary (`srunc`)                      (crates.io: srunc)
```

### Why this shape

- **The crypto is provably equivalent to Go.** `srun-core/tests/vectors.rs` pins XEncode / QuirkBase64Encode / `build_portal_params` byte-for-byte against the upstream Go output.
- **No global mutable state.** Timeout and HTTP agent live on `srun_client::Client`.
- **Typed errors / model.** `PortalError::from_code` returns `WrongPassword` / `AlreadyOnline` / … instead of `map[string]interface{}` + string compares.
- **`no_std` core** so it can target the smallest possible footprint.
- **Sync I/O via `ureq` + `rustls`, no tokio** — a single keep-alive timer doesn't justify an async runtime.
- **Zero deployment-specific knowledge** in the protocol code. `srun_client::discover` derives `(host, ac_id)` from the captive redirect; `profiles` is just a tiny built-in lookup for the rare cases where discovery misses.

## Build / test from source

```sh
cargo fmt --all -- --check                              # formatting gate
cargo clippy --all-targets --locked -- -D warnings      # lint gate
cargo test --locked --all-targets                       # 52 tests (5 vectors + 30 unit + 7 + 10 integration)
cargo build --release --locked                          # ~2.3 MB at target/release/srunc
```

CI runs all of these on every push / PR — see [`.github/workflows/ci.yml`](../.github/workflows/ci.yml).

## Cross-platform distribution

[`.github/workflows/release.yml`](../.github/workflows/release.yml) publishes archives + `SHA256SUMS` + `luci-app-srunc_*.ipk` to a GitHub Release on tag `v*`.

| OS | Targets |
|----|---------|
| Linux glibc + static musl | `x86_64`, `aarch64` |
| Linux static musl (routers) | `armv7-musleabihf`, `arm-musleabi` |
| macOS | `x86_64`, `aarch64` (Apple Silicon) |
| Windows | `x86_64-msvc` |

Linux cross targets are built with [`cross`](https://github.com/cross-rs/cross) (Docker-based). The musl targets are UPX-compressed post-build (~60% smaller, router-friendly). Size-tuned release profile: `opt-level="z"`, LTO, `panic="abort"`, `strip`.

### OpenWrt / MIPS (best-effort)

In Rust, `mipsel`/`mips` are tier-3 targets and need `-Z build-std` on nightly:

```sh
cargo +nightly build --release -Z build-std=std,panic_abort \
  --target mipsel-unknown-linux-musl
```

Most actively-supported OpenWrt devices in the past five years are armv7 / aarch64 (MT7621 / MT7986 / IPQ60xx) — the default matrix covers those. Add a `continue-on-error` mips job if you need legacy AR71xx hardware.

## Regenerating the end-to-end vector

The base64 / XEncode vectors are copied straight from `libauth/coding_test.go` in the upstream Go repo. The `build_portal_params` vector was produced from the Go reference. To regenerate, add a throwaway test in package `libauth` that calls `buildLoginParams("hello", "pass", "<token>", false, "", "1")` and prints `info` / `chksum`, then run `go test ./libauth -run <name> -v`.

## Adding a deployment profile

The profile registry is a compile-time slice in [`crates/srun/src/profiles.rs`](../crates/srun/src/profiles.rs) — pre-canned `(host, ac_id)` pairs for known srun customers (universities, banks, enterprises; see [srun.com](https://www.srun.com/) for the official customer list). To add one:

1. Append a `Profile { id, label, host, ac_id }` to `PROFILES`, keeping the slice alphabetical by `id`.
2. `id` is kebab-case and what users pass to `--profile`; `label` is the human-readable name (English + native script if useful); `host` and `ac_id` are the default portal endpoint + access-controller id for that deployment.
3. Only add a profile whose `host`/`ac_id` you have actually verified on the network — unverified entries are worse than no entry.
4. Re-run `cargo test` (the registry has a uniqueness check).
5. The OpenWrt LuCI Settings page picks up new profiles automatically via the `ubus call srunc profiles` rpcd method (no JS changes needed).

A remote/network profile list was considered and rejected: profiles change rarely, and a network dependency for a settings-page dropdown would add a meaningful supply-chain surface for little gain.
