# `packaging/` — native packaging that ships with this repo

All install paths below are produced from this repository's CI on each tag:

| Channel | User installs with | Built by |
|---|---|---|
| crates.io | `cargo install srunc` | `publish` job in `release.yml` |
| `cargo binstall` (prebuilt) | `cargo binstall srunc` | `[package.metadata.binstall]` in `Cargo.toml` + GH Release tarballs |
| GitHub Release tarball | `curl -L … \| tar xz` | `build` matrix in `release.yml` |
| Debian / Ubuntu `.deb` | `dpkg -i srunc_*.deb` | `[package.metadata.deb]` in `Cargo.toml` + `cargo-deb` step (gnu targets only) |
| OpenWrt `.ipk` | `opkg install luci-app-srunc_*.ipk` | `openwrt/gh-action-sdk` action, sourced from [`openwrt/luci-app-srunc/`](openwrt/luci-app-srunc/) |

We don't maintain a Homebrew tap, Scoop bucket, or AUR repo — each requires its own external git remote, and `cargo binstall` + raw tarball cover the realistic user base.
