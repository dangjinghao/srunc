# Releasing authu

This project ships through several channels. New tags trigger CI which builds per-target binaries + the `.ipk`; crates.io publishing is one manual step. Homebrew/AUR/OpenWrt-feed channels live in separate repos.

## Channels (current + planned)

| Channel | How users install | Status |
|---|---|---|
| **GitHub Releases** | Download a per-target `authu-<target>.tar.gz` from the [Releases page](https://github.com/wangyihang/authu/releases) | ✅ Live (CI: `release.yml`) |
| **crates.io** | `cargo install authu` | 🟡 metadata ready, run `cargo publish` to ship |
| **OpenWrt** `.ipk` | `opkg install luci-app-authu_*.ipk` (binary still from the tarball) | ✅ CI builds + attaches `.ipk` to each release |
| **Homebrew tap** | `brew install wangyihang/tap/authu` | ⏳ requires a separate `homebrew-tap` repo (formula pinned to release tarball SHAs) |
| **AUR** | `yay -S authu` (or similar) | ⏳ PKGBUILD pointing at GitHub release binary |
| **Own OpenWrt feed** | add `src-git authu https://github.com/wangyihang/authu;main` to `/etc/opkg/customfeeds.conf`, then `opkg update && opkg install luci-app-authu` | ⏳ requires `packaging/openwrt/Packages` index hosted somewhere |
| **OpenWrt official `packages` feed** | comes bundled with stock OpenWrt | ⏳ would require submitting a PR to [openwrt/packages](https://github.com/openwrt/packages); the Rust binary side needs `lang/rust` SDK build support |

## Cutting a release

1. Bump `workspace.package.version` in `Cargo.toml` to the new version (e.g. `0.2.0`).
2. Bump the pinned versions in path-deps too — `srun = { path = "...", version = "0.2.0" }` and the same for `srun-core`. (cargo refuses to publish without explicit versions on path-deps.)
3. Update `packaging/openwrt/luci-app-authu/Makefile` `PKG_VERSION` to match.
4. `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, `cargo test --all-targets` — all green.
5. Commit, tag `vX.Y.Z`, push tag — `release.yml` builds per-target binaries + the `.ipk` and attaches everything to the GitHub Release.
6. Publish the Rust crates (see below).

## Publishing to crates.io

Three crates are published:

| Crate | What it is | Depends on |
|---|---|---|
| [`srun-core`](https://crates.io/crates/srun-core) | pure `no_std` protocol crypto | (none) |
| [`srun-client`](https://crates.io/crates/srun-client) | sync HTTP client + auto-discovery | `srun-core` |
| [`authu`](https://crates.io/crates/authu) | the CLI binary | `srun-core` + `srun-client` |

> **Note on naming**: the workspace dir is `crates/srun/` but the published crate is `srun-client` because `srun` is already taken on crates.io (an unrelated sandbox tool). End users `cargo install authu` for the binary; library users add `srun-client = "0.1"`.

Dependencies must be published before dependents. The order is fixed:

```sh
cargo publish -p srun-core
# wait ~30 s for the index to update before the next crate can resolve it
cargo publish -p srun-client
cargo publish -p authu
```

You can dry-run any of these with `--dry-run` first (the network step is skipped and the package is just verified locally).

A `cargo login <token>` is needed once; the token is created at <https://crates.io/me>.

## Homebrew tap (planned)

Create a `wangyihang/homebrew-tap` repo with `Formula/authu.rb`:

```ruby
class Authu < Formula
  desc "Generic auto-configuring srun (深澜) campus-portal auth CLI"
  homepage "https://github.com/wangyihang/authu"
  version "0.1.0"
  license "GPL-3.0-or-later"

  on_macos do
    on_arm do
      url "https://github.com/wangyihang/authu/releases/download/v#{version}/authu-aarch64-apple-darwin.tar.gz"
      sha256 "..."
    end
    on_intel do
      url "https://github.com/wangyihang/authu/releases/download/v#{version}/authu-x86_64-apple-darwin.tar.gz"
      sha256 "..."
    end
  end

  on_linux do
    on_arm { ... }
    on_intel do
      url "https://github.com/wangyihang/authu/releases/download/v#{version}/authu-x86_64-unknown-linux-musl.tar.gz"
      sha256 "..."
    end
  end

  def install
    bin.install "authu"
  end

  test do
    system "#{bin}/authu", "--version"
  end
end
```

Bump on each release by pasting the new SHAs from `dist/SHA256SUMS`.

## AUR (planned)

`PKGBUILD` referencing the release tarball — see [Arch Wiki: PKGBUILD](https://wiki.archlinux.org/title/PKGBUILD).
