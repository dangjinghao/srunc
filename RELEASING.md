# Releasing srunc

Everything ships out of this one repo. A tag push to `v*` triggers `release.yml`, which builds per-target binaries, the OpenWrt `.ipk`, and the Debian `.deb`, then uploads everything to a GitHub Release with `SHA256SUMS`. Publishing the Rust crates to crates.io is the only manual step.

## Install channels

| Channel | How users install | Built by |
|---|---|---|
| GitHub Releases | Download a per-target `srunc-<target>.tar.gz` from the [Releases page](https://github.com/WangYihang/srunc/releases) | `release.yml` build matrix |
| `cargo binstall` | `cargo binstall srunc` (prebuilt binary, no compile) | `[package.metadata.binstall]` + the same tarballs |
| `cargo install` | `cargo install srunc` (compiles from source) | `cargo publish` after each tag |
| OpenWrt `.ipk` | `opkg install luci-app-srunc_*.ipk` | `openwrt/gh-action-sdk` in `release.yml` |
| Debian / Ubuntu `.deb` | `dpkg -i srunc_*.deb` | `cargo-deb` step in `release.yml` (gnu targets only) |

That's all five paths. We deliberately don't maintain a Homebrew tap, AUR repo, or Scoop bucket — each would require its own external git remote, and `cargo binstall` plus the raw tarball already cover those users without spreading the project across multiple repos.

## Cutting a release

1. Bump `workspace.package.version` in `Cargo.toml` to the new version (e.g. `0.3.0`).
2. Bump the pinned versions in path-deps too — `srun-core = { path = "...", version = "0.3.0" }` and the same for `srun-client`. Cargo refuses to publish without explicit versions on path-deps.
3. Update `packaging/openwrt/luci-app-srunc/Makefile` `PKG_VERSION` to match.
4. `cargo fmt --all`, `cargo clippy --all-targets --locked -- -D warnings`, `cargo test --all-targets --locked` — all green.
5. Commit, tag `vX.Y.Z`, push tag. `release.yml` builds per-target binaries + the `.ipk` + the `.deb` and attaches everything to the GitHub Release with `SHA256SUMS`.
6. Publish the Rust crates to crates.io (see below).

## Publishing to crates.io

Three crates are published:

| Crate | What it is | Depends on |
|---|---|---|
| [`srun-core`](https://crates.io/crates/srun-core) | pure `no_std` protocol crypto | (none) |
| [`srun-client`](https://crates.io/crates/srun-client) | sync HTTP client + auto-discovery | `srun-core` |
| [`srunc`](https://crates.io/crates/srunc) | the CLI binary | `srun-core` + `srun-client` |

> **Note on naming**: the workspace dir is `crates/srun/` but the published crate is `srun-client` because `srun` is already taken on crates.io (an unrelated sandbox tool). End users run `cargo install srunc` for the binary; library users add `srun-client = "0.x"`.

Dependencies must be published before dependents. The order is fixed:

```sh
cargo publish -p srun-core
# wait ~30 s for the index to update before the next crate can resolve it
cargo publish -p srun-client
cargo publish -p srunc
```

Dry-run with `--dry-run` first; the network upload is skipped and the package is verified locally.

`cargo login <token>` is needed once; the token is created at <https://crates.io/me>.
