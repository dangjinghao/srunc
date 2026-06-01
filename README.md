# authu — Rust implementation of the srun (深澜) campus-portal auth

A Rust authenticator for the **srun** captive portal used by TUNET
(`auth4.tsinghua.edu.cn` / `auth6.tsinghua.edu.cn`) and many other Chinese
universities' campus networks. Designed for unattended use on servers and
routers — small static binary, runs cleanly under systemd / OpenWRT init.d.

This is a clean-room Rust port of the auth core from
[`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing) (via
[`WangYihang/GoAuthing`](https://github.com/WangYihang/GoAuthing)),
licensed under the same GPLv3.

## Quick start

```sh
# 1. Install. Pick one:
cargo install authu                              # from crates.io
# or grab a per-target musl binary from the latest GitHub Release,
# or `cargo build --release` from source.

# 2. Write a config file — username + password is enough; host and ac_id
#    are auto-discovered from the captive-portal redirect.
mkdir -p ~/.config/authu
cat > ~/.config/authu/config.toml <<'EOF'
username = "your-tunet-account"
password = "your-tunet-password"
EOF

# 3. Use it.
authu login              # one-shot auth (auto-discovers host + ac_id)
authu status             # human-readable status
authu status --json      # machine-readable {"online":bool,"username":...,"ip":...}
authu login -k           # auth + keep the session alive (Ctrl-C to stop)
authu logout
authu -v status          # bump verbosity to see protocol-level debug logs
```

On OpenWrt: install [`luci-app-authu`](packaging/openwrt/) (the `.ipk` is attached to each GitHub Release) for a web UI under *Services › authu*.

Anything in the config can be overridden on the command line — `--ipv6/-6`,
`--ac-id`, `--host`, `--insecure`, `--timeout/-t`, `--username/-u`,
`--password/-p`, `--config/-c`, `--json`, `-v/-q`. Run `authu --help` for the full surface.

## Configuration

TOML, located at one of (first match wins):

1. `--config <path>`
2. `$AUTHU_CONFIG`
3. `$XDG_CONFIG_HOME/authu/config.toml`
4. `~/.config/authu/config.toml`
5. `~/.authu.toml`

Most users only need `username` + `password`. Full example in
[`docs/config.example.toml`](docs/config.example.toml).

## Service mode

`--daemonize / -D` skips stdin prompts and only emits ERROR-level logs —
intended for systemd / init.d. Service files for the common targets:

| Platform | File |
|----------|------|
| systemd (system-wide) | [`docs/systemd/system/authu.service`](docs/systemd/system/authu.service) — reads `/etc/authu/config.toml` |
| systemd (templated) | [`docs/systemd/system/authu@.service`](docs/systemd/system/authu@.service) — `/etc/authu/<name>.toml` per instance |
| systemd (per-user) | [`docs/systemd/user/authu.service`](docs/systemd/user/authu.service) — `~/.config/authu/config.toml` |
| OpenWRT (procd) | [`docs/openwrt/init.d/authu`](docs/openwrt/init.d/authu) — drop in `/etc/init.d/` |

These are bundled in every release tarball under `docs/` so you can drop them
straight onto the target box.

## Architecture

**Functional core, imperative shell** — the hard, error-prone reverse-engineered
crypto is a pure, side-effect-free crate; HTTP and HTML scraping live in a thin
replaceable I/O layer.

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
│   └── src/tsinghua.rs   legacy DNS-based ac_id detection (fallback)
│
└── srun-cli/         this binary (`authu`)                      (crates.io: authu)
```

Why it is shaped this way:

- **The crypto is provably equivalent to Go.** `srun-core/tests/vectors.rs`
  pins XEncode / QuirkBase64Encode / build_portal_params byte-for-byte against
  the upstream Go output.
- **No global mutable state.** Timeout and HTTP agent live on `srun_client::Client`.
- **Typed errors / model** — `PortalError::from_code` → `WrongPassword` /
  `AlreadyOnline` / … instead of `map[string]interface{}` + string compares.
- **`no_std` core** so it can target the smallest possible footprint.
- **Sync I/O via `ureq` + `rustls`, no tokio** — a single keep-alive timer
  doesn't justify an async runtime.
- **Site quirks isolated** in `srun_client::tsinghua` so the protocol code stays
  portable to other srun deployments.

## Build / test / run from source

```sh
cargo fmt --all -- --check                              # formatting gate
cargo clippy --all-targets --locked -- -D warnings      # lint gate
cargo test --locked                                     # 5 vector + 8 unit = 13 tests
cargo build --release --locked                          # ~2.2 MB at target/release/authu
cargo run -- login -k                                   # if you have a config
```

CI on every push / PR runs the first three commands; see
[`.github/workflows/ci.yml`](.github/workflows/ci.yml).

## Cross-platform distribution

`.github/workflows/release.yml` publishes archives + `SHA256SUMS` to a GitHub
Release on tag `v*` (or via `workflow_dispatch`). Each tarball ships the binary,
`LICENSE`, `README.md`, and `docs/` (so service files come along for the ride).

| OS | Targets |
|----|---------|
| Linux (glibc + static musl) | `x86_64`, `aarch64` |
| Linux (static musl, routers) | `armv7-musleabihf`, `arm-musleabi` |
| macOS | `x86_64`, `aarch64` (Apple Silicon) |
| Windows | `x86_64-msvc` |

Linux cross targets are built with [`cross`](https://github.com/cross-rs/cross)
(Docker-based), so no host cross-toolchains are required. The size-tuned
release profile (`opt-level="z"`, LTO, `panic="abort"`, `strip`) keeps
artifacts small.

### OpenWRT / MIPS (best-effort)

In Rust, `mipsel`/`mips` are tier-3 targets and need `-Z build-std` on nightly:

```sh
cargo +nightly build --release -Z build-std=std,panic_abort \
  --target mipsel-unknown-linux-musl
```

Most actively-supported OpenWRT devices in the past five years are armv7 /
aarch64 (MT7621 / MT7986 / IPQ60xx) — the default matrix covers those. If you
need mips for legacy AR71xx-class hardware, add a `continue-on-error` job.

## Regenerating the end-to-end vector

The base64 / XEncode vectors are copied straight from `libauth/coding_test.go`
in the upstream Go repo. The `build_portal_params` vector was produced from the
Go reference. To regenerate, add a throwaway test in package `libauth` that
calls `buildLoginParams("hello", "pass", "<token>", false, "", "1")` and prints
`info` / `chksum`, then run `go test ./libauth -run <name> -v`.

## License

GPL-3.0-or-later — same as the upstream
[`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing) project this is derived
from.
