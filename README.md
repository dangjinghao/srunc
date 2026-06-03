# srunc

Tiny static-binary client for **srun** (深澜) captive portals.

## Install

Pick the line for your system. Replace `VERSION` with the latest tag from
[Releases](https://github.com/WangYihang/srunc/releases).

### OpenWrt (router, with LuCI UI)

```sh
opkg install luci-app-srunc_*.ipk
```

Download the `.ipk` matching your router's arch from the release page
(`x86_64`, `aarch64_cortex-a53/a72/generic`, `arm_cortex-a7_neon-vfpv4`).

### Debian / Ubuntu

```sh
curl -LO https://github.com/WangYihang/srunc/releases/download/vVERSION/srunc_VERSION-1_amd64.deb
sudo dpkg -i srunc_VERSION-1_amd64.deb
```

`arm64` builds are published alongside.

### macOS / Windows / other Linux

Prebuilt binary:

```sh
cargo binstall srunc
```

Or grab the tarball/zip for your target from the release page and drop the
`srunc` binary in your `PATH`.

### From source

```sh
cargo install srunc
```

## Usage

```sh
srunc login          # prompts for username + password
srunc login -k       # keep the session alive (re-auth on drop)
srunc status
srunc logout
```

Host and `ac_id` are auto-discovered from the captive redirect — most networks
need nothing more than username + password. To skip the prompt, drop a config
at `~/.config/srunc/config.toml` (see [`docs/config.example.toml`](docs/config.example.toml)).
