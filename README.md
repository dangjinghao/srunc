# authu — srun (深澜) campus-portal auth

Small static-binary authenticator for the **srun** (深澜) captive portal used by TUNET and many other Chinese university networks. Designed for headless servers and routers. Auto-discovers `host` + `ac_id` from the captive redirect; most setups only need username + password.

> Name: `auth` + `thu` (Tsinghua, where the project got its start). The code is deployment-agnostic.

## Install

```sh
cargo binstall authu      # prebuilt binary from the latest release (fast)
cargo install authu       # compile from source
```

Or grab a per-target binary from the [latest Release](https://github.com/WangYihang/authu/releases) directly. On OpenWrt, install [`luci-app-authu`](packaging/openwrt/) for a web UI under *Services › authu*. Per-distro packages: see [`packaging/`](packaging/).

## Configure

Write `~/.config/authu/config.toml`:

```toml
username = "your-account"
password = "your-password"
```

Full options + search paths: [`docs/config.example.toml`](docs/config.example.toml).

## Run

```sh
authu login              # one-shot auth
authu login -k           # auth + keep the session alive
authu status             # online? user? ip?
authu status --json      # machine-readable
authu logout
authu --help             # all flags (-v / -q / --host / --ac-id / ...)
```

## Service mode

`-D / --daemonize` skips stdin prompts and only logs ERROR. Drop-in units:

| Platform | File |
|----------|------|
| systemd (system) | [`docs/systemd/system/authu.service`](docs/systemd/system/authu.service) |
| systemd (templated, multi-instance) | [`docs/systemd/system/authu@.service`](docs/systemd/system/authu@.service) |
| systemd (per-user) | [`docs/systemd/user/authu.service`](docs/systemd/user/authu.service) |
| OpenWrt LuCI `.ipk` | [`packaging/openwrt/`](packaging/openwrt/) |

## More

- Internals (architecture, build, cross-compile, regenerating vectors): [`docs/internals.md`](docs/internals.md)
- Release & publish process: [`RELEASING.md`](RELEASING.md)
- Clean-room Rust port of [`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing). License: GPL-3.0-or-later.
