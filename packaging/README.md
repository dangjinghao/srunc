# `packaging/` — per-distro templates

Canonical, version-controlled templates for each downstream distribution. Each gets pushed to **its own publishing repo** (Homebrew tap, AUR git, Scoop bucket), but lives here as the source of truth so a single bump script keeps them all in sync.

```
packaging/
├── homebrew/authu.rb            ← published to:  WangYihang/homebrew-tap, Formula/authu.rb
├── aur/{PKGBUILD,.SRCINFO}      ← published to:  ssh://aur@aur.archlinux.org/authu-bin.git
├── scoop/authu.json             ← published to:  WangYihang/scoop-bucket, bucket/authu.json
└── openwrt/luci-app-authu/      ← built into a .ipk by CI on every release tag
```

Other distribution channels live elsewhere (no per-distro file needed):

| Channel | Built by | User installs with |
|---|---|---|
| **crates.io** (`cargo install authu`) | `cargo publish` (manual, in release order srun-core → srun-client → authu) | `cargo install authu` |
| **cargo binstall** (prebuilt from GH Release) | metadata in `crates/srun-cli/Cargo.toml` `[package.metadata.binstall]` | `cargo binstall authu` |
| **Debian / Ubuntu `.deb`** | `cargo deb` in `release.yml`, attached to GH Release | `dpkg -i authu_*_amd64.deb` |
| **OpenWrt `.ipk`** | `openwrt/gh-action-sdk` in `release.yml`, attached to GH Release | `opkg install luci-app-authu_*.ipk` |
| **GitHub Release tarballs** | `release.yml` per-target build matrix | `curl -L … | tar xz` |

## Release flow (per tag)

1. **Tag pushed** → CI builds binaries + `.deb` + `.ipk` + uploads them to a GitHub Release with `SHA256SUMS`.
2. **`scripts/bump-packaging.sh <version>`** reads `SHA256SUMS` from that release and rewrites the version + sha256 strings in:
   - `homebrew/authu.rb`
   - `aur/PKGBUILD` (regenerates `.SRCINFO` if `makepkg` is on `$PATH`, else prints a reminder)
   - `scoop/authu.json`
3. **Sync each updated file into its publishing repo** (`git add && git commit && git push`):
   - `homebrew/authu.rb` → `WangYihang/homebrew-tap` (`Formula/authu.rb`)
   - `aur/PKGBUILD` + `.SRCINFO` → AUR git remote
   - `scoop/authu.json` → `WangYihang/scoop-bucket` (`bucket/authu.json`)

Most of step 3 can be automated with GitHub Actions:

- Homebrew: [`dawidd6/action-homebrew-bump-formula`](https://github.com/dawidd6/action-homebrew-bump-formula)
- AUR: [`KSXGitHub/github-actions-deploy-aur`](https://github.com/KSXGitHub/github-actions-deploy-aur)
- Scoop: hand-roll a small action that commits to the bucket repo

That's the v0.3.0+ work if/when manual syncing becomes annoying.

## Adding a new distribution

The pattern:

1. Drop a template under `packaging/<distro>/`.
2. Teach `scripts/bump-packaging.sh` to rewrite the SHA / version in it.
3. Document the publishing repo path here.

For a one-off distro that doesn't need version bumps (e.g. a Nixpkgs PR), just maintain it directly in nixpkgs.
