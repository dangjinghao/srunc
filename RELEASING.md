# Releasing srunc

A `v*` tag push to `release.yml` builds per-target binaries, the OpenWrt `.ipk`, the Debian `.deb`, attaches everything to a GitHub Release with `SHA256SUMS`, **and** publishes the crate to crates.io. End-to-end automation; the only manual steps are bumping the version and pushing the tag.

## Install channels

| Channel | How users install | Built by |
|---|---|---|
| GitHub Releases | per-target `srunc-<target>.tar.gz` from the [Releases page](https://github.com/WangYihang/srunc/releases) | `release.yml` build matrix |
| `cargo binstall` | `cargo binstall srunc` | `[package.metadata.binstall]` + the same tarballs |
| `cargo install` | `cargo install srunc` | `cargo publish` in the `publish` job |
| OpenWrt `.ipk` | `opkg install luci-app-srunc_*.ipk` | `openwrt/gh-action-sdk` in `release.yml` |
| Debian / Ubuntu `.deb` | `dpkg -i srunc_*.deb` | `cargo-deb` step in `release.yml` (gnu targets only) |

## Cutting a release

1. Bump `version` in `Cargo.toml` (e.g. `0.4.0`).
2. Bump `PKG_VERSION` in `packaging/openwrt/luci-app-srunc/Makefile` to match.
3. `cargo fmt --check && cargo clippy --all-targets --locked -- -D warnings && cargo test --locked --all-targets` — all green.
4. Commit, tag `vX.Y.Z`, push tag. CI does the rest.

## One-time setup for crates.io publishing

1. Create a crates.io API token at <https://crates.io/me> (scope: `publish-new + publish-update`).
2. Add it to this repo's GitHub secrets as `CARGO_REGISTRY_TOKEN` (Settings → Secrets and variables → Actions).

[`katyo/publish-crates@v2`](https://github.com/katyo/publish-crates) handles the actual publish — it skips already-uploaded versions, so re-running a partially-failed workflow is safe.
