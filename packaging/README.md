# `packaging/` — native packaging that ships with this repo

Distribution channels are deliberately kept to **one repo**. All install paths below are produced from this repository's CI on each tag, attached to the GitHub Release, or live in `crates/`:

| Channel | User installs with | Built by |
|---|---|---|
| crates.io (source) | `cargo install srunc` | `cargo publish` (manual after each tag — see `RELEASING.md`) |
| `cargo binstall` (prebuilt) | `cargo binstall srunc` | `[package.metadata.binstall]` in `crates/srun-cli/Cargo.toml` + GH Release tarballs |
| GitHub Release tarball | `curl -L … \| tar xz` | `release.yml` build matrix |
| Debian / Ubuntu `.deb` | `dpkg -i srunc_*.deb` | `[package.metadata.deb]` in `crates/srun-cli/Cargo.toml` + `cargo-deb` step in `release.yml`; only for the `*-gnu` targets |
| OpenWrt `.ipk` | `opkg install luci-app-srunc_*.ipk` | `openwrt/gh-action-sdk` action in `release.yml`, sourced from [`openwrt/luci-app-srunc/`](openwrt/luci-app-srunc/) |

That's it. We don't maintain a Homebrew tap, a Scoop bucket, or an AUR repo — each of those requires its own external git remote to be useful, and the channels above already cover the realistic user base.

Power users on macOS/Arch/Windows can still install via `cargo binstall srunc` (no extra ecosystem repo needed) or the GitHub Release tarball.
