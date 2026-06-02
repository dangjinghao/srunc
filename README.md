# srunc

Tiny static-binary client for **srun** (深澜) captive portals — TUNet and many other Chinese university / corporate networks. Auto-discovers `host` + `ac_id` from the captive redirect; most users only need to set username + password.

```sh
cargo binstall srunc       # prebuilt; fast
cargo install srunc        # from source

srunc login                # one-shot
srunc login -k             # auth + keepalive
srunc status
```

Config: `~/.config/srunc/config.toml` (see [`docs/config.example.toml`](docs/config.example.toml)).

**OpenWrt**: install [`luci-app-srunc`](packaging/openwrt/) for a LuCI UI under *Services › srunc*.
**systemd**: drop-in units in [`docs/systemd/`](docs/systemd/).

More: [internals](docs/internals.md) · [releasing](RELEASING.md). License: GPL-3.0-or-later. Clean-room Rust port of [`z4yx/GoAuthing`](https://github.com/z4yx/GoAuthing).
