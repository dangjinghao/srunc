# srunc — srun (深澜) captive-portal auth

Small static-binary authenticator for the **srun** (深澜) captive portal used by universities (Tsinghua, Fudan, Zhejiang, …), banks, and corporate networks across China. Designed for headless servers and routers. Auto-discovers `host` + `ac_id` from the captive redirect; most setups only need username + password.

## Install

```sh
cargo binstall srunc      # prebuilt binary from the latest release (fast)
cargo install srunc       # compile from source
```

Or grab a per-target binary from the [latest Release](https://github.com/WangYihang/srunc/releases) directly. On OpenWrt, install [`luci-app-srunc`](packaging/openwrt/) for a web UI under *Services › srunc*. Per-distro packages: see [`packaging/`](packaging/).

## Configure

Write `~/.config/srunc/config.toml`:

```toml
username = "your-account"
password = "your-password"
```

Full options + search paths: [`docs/config.example.toml`](docs/config.example.toml).

## Run

```sh
srunc login              # one-shot auth
srunc login -k           # auth + keep the session alive
srunc status             # online? user? ip?
srunc status --json      # machine-readable
srunc logout
srunc profiles list      # built-in host + ac_id profiles (e.g. tsinghua)
srunc --profile tsinghua login  # pick a profile instead of overriding fields
srunc --help             # all flags (-v / -q / --host / --ac-id / ...)
```

## Service mode

`-D / --daemonize` skips stdin prompts and only logs ERROR. Drop-in units:

| Platform | File |
|----------|------|
| systemd (system) | [`docs/systemd/system/srunc.service`](docs/systemd/system/srunc.service) |
| systemd (templated, multi-instance) | [`docs/systemd/system/srunc@.service`](docs/systemd/system/srunc@.service) |
| systemd (per-user) | [`docs/systemd/user/srunc.service`](docs/systemd/user/srunc.service) |
| OpenWrt LuCI `.ipk` | [`packaging/openwrt/`](packaging/openwrt/) |

## More

- Internals (architecture, build, cross-compile, regenerating vectors): [`docs/internals.md`](docs/internals.md)
- Release & publish process: [`RELEASING.md`](RELEASING.md)
- Clean-room Rust port of [`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing). License: GPL-3.0-or-later.
