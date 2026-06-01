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

Step 3 is **automated** via [`.github/workflows/sync-packaging.yml`](../.github/workflows/sync-packaging.yml) — fires on `release: published` (i.e. after `release.yml` finishes uploading assets), re-runs the bump script against the just-published `SHA256SUMS`, then pushes each template to its destination repo in parallel. AUR side uses [`KSXGitHub/github-actions-deploy-aur`](https://github.com/KSXGitHub/github-actions-deploy-aur) which runs `makepkg --printsrcinfo` inside an Arch container to regenerate `.SRCINFO`.

### One-time setup for the auto-sync workflow

You need three external repos and two GitHub secrets configured on this repo. Until both are set, the sync jobs will fail (the release itself is unaffected).

**1. External repos** (create empty + initialize with a README):

| Repo | What sync-packaging.yml expects |
|---|---|
| `WangYihang/homebrew-tap` | the formula lands at `Formula/authu.rb` |
| `WangYihang/scoop-bucket` | the manifest lands at `bucket/authu.json` |
| `aur@aur.archlinux.org/authu-bin.git` | created via [aur.archlinux.org](https://aur.archlinux.org) after registering an account + uploading an SSH pubkey under *My Account → Edit → SSH Public Key* |

**2. Secrets** (repo *Settings → Secrets and variables → Actions → New repository secret*):

| Secret name | Value |
|---|---|
| `PACKAGING_PAT` | a [fine-grained PAT](https://github.com/settings/personal-access-tokens/new) with **Contents: Read and write** on both `homebrew-tap` and `scoop-bucket`; no other permissions needed |
| `AUR_SSH_PRIVATE_KEY` | the private half of the SSH key whose pubkey you added to your AUR account. Generate with `ssh-keygen -t ed25519 -N "" -f aur_key`, copy `aur_key.pub` content to AUR, and paste the whole `aur_key` (including `-----BEGIN…-----` lines) into this secret |

**3. User-facing install commands** once the three repos exist:

```sh
brew install WangYihang/tap/authu
yay -S authu-bin                  # or paru / pacman wrapper of choice
scoop bucket add wangyihang https://github.com/WangYihang/scoop-bucket && scoop install authu
```

### Manually triggering a sync

If a sync job fails or you want to re-run for an existing tag: Actions tab → *Sync packaging* → *Run workflow* → enter version (no leading `v`). It re-runs the bump script against the canonical `SHA256SUMS` and pushes idempotently — no-op if the destination is already current.

## Adding a new distribution

The pattern:

1. Drop a template under `packaging/<distro>/`.
2. Teach `scripts/bump-packaging.sh` to rewrite the SHA / version in it.
3. Document the publishing repo path here.

For a one-off distro that doesn't need version bumps (e.g. a Nixpkgs PR), just maintain it directly in nixpkgs.
