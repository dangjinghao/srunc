# authu — Rust implementation of the srun (深澜) campus-portal auth

A Rust authenticator for the **srun** captive portal used by TUNET
(`auth4.tsinghua.edu.cn` / `auth6.tsinghua.edu.cn`) and many other Chinese
universities' campus networks. Structured as **functional core + imperative
shell** so the reverse-engineered crypto lives in a pure, testable library, and
the I/O is a thin replaceable layer on top.

This is a clean-room Rust port of the auth core from
[`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing) (via
[`WangYihang/GoAuthing`](https://github.com/WangYihang/GoAuthing)) and is
licensed under the same GPLv3.

## Why this layout

The hard, error-prone part of this kind of captive-portal auth is *pure
computation* (the reverse-engineered srun crypto). It lives in its own
side-effect-free crate that is pinned byte-for-byte to the Go reference output:

```
crates/
├── srun-core/        # PURE, no_std, zero I/O, zero globals
│   ├── src/xencode.rs    # XEncode — srun's XXTEA-variant block cipher
│   ├── src/base64.rs     # QuirkBase64Encode (custom alphabet)
│   ├── src/digest.rs     # MD5 / SHA1 hex (RustCrypto)
│   ├── src/params.rs     # build_portal_params (login/logout assembly)
│   ├── src/error.rs      # typed PortalError + full code→message table
│   └── tests/vectors.rs  # golden vectors pinned to the Go reference
└── srun-cli/         # thin std shell: `params` (offline) + `selftest`
```

What this buys:

- **The crypto is provably equivalent to the Go reference.**
  `tests/vectors.rs` reproduces the XEncode / QuirkBase64 vectors from
  `libauth/coding_test.go` plus a full end-to-end `build_portal_params` result
  byte-for-byte.
- **No global mutable state** (Go's `libauth.HttpTimeout` package var becomes a
  client field once the I/O shell lands).
- **Typed errors / model** instead of `map[string]interface{}` + string compares
  (`PortalError::from_code` → `WrongPassword`, `AlreadyOnline`, …).
- **`no_std` core** so the footprint stays tiny.

## Not done yet — the imperative shell

The networking layer (`get_challenge` → `login`/`logout` → keep-alive, plus the
Tsinghua-specific `ac_id` HTML scraping) is intentionally separate. It will be a
`srun` crate depending on `srun-core`, using a sync HTTP client
(`ureq` + `rustls`, *not* tokio: a single keep-alive timer doesn't need an async
runtime) with `serde` response structs and `context`-style cancellation.

## Build / test / run

```sh
cargo test                      # runs the golden-vector suite
cargo build --release           # ~0.5 MB binary; produces target/release/authu
cargo run -p srun-cli -- selftest
cargo run -p srun-cli -- params -u <user> -p <pass> -t <challenge-token> --ac-id 1
```

Reference numbers: release binary ~**509 KB** (glibc) /
~**593 KB statically linked** for `x86_64-unknown-linux-musl`.

## Cross-platform distribution

`.github/workflows/release.yml` builds a matrix on tag `v*` (or
`workflow_dispatch`) and publishes archives + `SHA256SUMS` to a GitHub Release:

| OS | Targets |
|----|---------|
| Linux (glibc + static musl) | `x86_64`, `aarch64` |
| Linux (static musl, routers) | `armv7-musleabihf`, `arm-musleabi` |
| macOS | `x86_64`, `aarch64` (Apple Silicon) |
| Windows | `x86_64-msvc` |

Linux cross targets are built with [`cross`](https://github.com/cross-rs/cross)
(Docker-based), so no host cross-toolchains are needed. The size-tuned release
profile (`opt-level="z"`, LTO, `panic="abort"`, `strip`) keeps artifacts small.

### OpenWRT / MIPS (best-effort)

In Rust, `mipsel`/`mips` are tier-3 targets and need `-Z build-std` on nightly:

```sh
cargo +nightly build --release -Z build-std=std,panic_abort \
  --target mipsel-unknown-linux-musl
```

They're left out of the default matrix to keep CI reliably green; add them as a
`continue-on-error` job if you need router binaries.

## Regenerating the end-to-end vector

The base64/XEncode vectors come straight from `libauth/coding_test.go` in the
upstream Go repo. The `build_portal_params` vector was produced from the Go
reference. To regenerate, add a throwaway test in package `libauth` that calls
`buildLoginParams("hello", "pass", "<token>", false, "", "1")` and prints
`info` / `chksum`, then run `go test ./libauth -run <name> -v`.

## License

GPL-3.0-or-later — same as the upstream
[`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing) project this is derived
from.
