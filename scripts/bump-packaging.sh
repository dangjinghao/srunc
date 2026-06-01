#!/usr/bin/env bash
# Refresh version + SHA256s across the per-distro packaging templates
# (Homebrew, AUR, Scoop) using the canonical SHA256SUMS from the GitHub
# Release of the given tag.
#
# Usage:  scripts/bump-packaging.sh 0.3.0
#
# After running, sync the updated files into their respective publishing
# repos (homebrew tap repo, AUR git repo, scoop bucket repo).

set -euo pipefail

VERSION="${1:-}"
if [ -z "$VERSION" ]; then
    echo "usage: $0 <version>   e.g. $0 0.3.0" >&2
    exit 1
fi

REPO_ROOT="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"
URL_BASE="https://github.com/WangYihang/authu/releases/download/v${VERSION}"
TMP=$(mktemp -d)
trap 'rm -rf "$TMP"' EXIT

echo "Fetching SHA256SUMS for v${VERSION}..."
# Prefer `gh` (uses GitHub API, works behind networks that block the asset CDN
# and authenticates automatically for private repos). Fall back to curl for
# environments without gh installed.
if command -v gh >/dev/null; then
    gh release download "v${VERSION}" --repo WangYihang/authu \
        --pattern SHA256SUMS --output "${TMP}/SHA256SUMS" --clobber
else
    curl -fsSL "${URL_BASE}/SHA256SUMS" >"${TMP}/SHA256SUMS"
fi

# Pull a single artifact's SHA from the SHA256SUMS file.
sha() { awk -v f="$1" '$2 == f { print $1 }' "${TMP}/SHA256SUMS"; }

SHA_DARWIN_ARM=$(sha   "authu-aarch64-apple-darwin.tar.gz")
SHA_DARWIN_INTEL=$(sha "authu-x86_64-apple-darwin.tar.gz")
SHA_MUSL_X86_64=$(sha  "authu-x86_64-unknown-linux-musl.tar.gz")
SHA_MUSL_AARCH64=$(sha "authu-aarch64-unknown-linux-musl.tar.gz")
SHA_MUSL_ARMV7=$(sha   "authu-armv7-unknown-linux-musleabihf.tar.gz")
SHA_MUSL_ARM=$(sha     "authu-arm-unknown-linux-musleabi.tar.gz")
SHA_WIN_X86_64=$(sha   "authu-x86_64-pc-windows-msvc.zip")

for v in DARWIN_ARM DARWIN_INTEL MUSL_X86_64 MUSL_AARCH64 MUSL_ARMV7 MUSL_ARM WIN_X86_64; do
    var="SHA_${v}"
    if [ -z "${!var}" ]; then
        echo "error: missing SHA for $var in SHA256SUMS — did the release publish all expected artifacts?" >&2
        exit 1
    fi
done

cd "$REPO_ROOT"

# ---------- Homebrew formula ----------
brew=packaging/homebrew/authu.rb
python3 - "$brew" "$VERSION" \
    "$SHA_DARWIN_ARM" "$SHA_DARWIN_INTEL" "$SHA_MUSL_AARCH64" "$SHA_MUSL_X86_64" <<'PY'
import re, sys
path, ver, d_arm, d_intel, l_arm, l_intel = sys.argv[1:]
src = open(path).read()
src = re.sub(r'version "[^"]+"', f'version "{ver}"', src, count=1)

def replace_block(text, marker_substr, new_sha):
    # find `url "....<marker>...."` block then the immediately following `sha256 "..."` and replace.
    pat = re.compile(
        r'(url "[^"]*' + re.escape(marker_substr) + r'[^"]*"\s*\n\s*sha256 ")[0-9a-f]+(")'
    )
    return pat.sub(r'\g<1>' + new_sha + r'\g<2>', text, count=1)

src = replace_block(src, 'aarch64-apple-darwin.tar.gz',           d_arm)
src = replace_block(src, 'x86_64-apple-darwin.tar.gz',            d_intel)
src = replace_block(src, 'aarch64-unknown-linux-musl.tar.gz',     l_arm)
src = replace_block(src, 'x86_64-unknown-linux-musl.tar.gz',      l_intel)
open(path, 'w').write(src)
PY
echo "✓ packaging/homebrew/authu.rb"

# ---------- AUR PKGBUILD ----------
aur=packaging/aur/PKGBUILD
python3 - "$aur" "$VERSION" \
    "$SHA_MUSL_X86_64" "$SHA_MUSL_AARCH64" "$SHA_MUSL_ARMV7" "$SHA_MUSL_ARM" <<'PY'
import re, sys
path, ver, s_x, s_a, s_v7, s_arm = sys.argv[1:]
src = open(path).read()
src = re.sub(r'^pkgver=.*$', f'pkgver={ver}', src, count=1, flags=re.MULTILINE)
src = re.sub(r'^pkgrel=.*$', 'pkgrel=1',      src, count=1, flags=re.MULTILINE)
for arch, sha in [('x86_64', s_x), ('aarch64', s_a), ('armv7h', s_v7), ('arm', s_arm)]:
    src = re.sub(
        rf"^(sha256sums_{arch}=\(')[0-9a-f]+('\))$",
        rf"\g<1>{sha}\g<2>",
        src, count=1, flags=re.MULTILINE,
    )
open(path, 'w').write(src)
PY
echo "✓ packaging/aur/PKGBUILD"

# .SRCINFO needs `makepkg --printsrcinfo` to regenerate. If makepkg is on the
# host, do it; otherwise leave a note for the human.
if command -v makepkg >/dev/null; then
    (cd packaging/aur && makepkg --printsrcinfo >.SRCINFO)
    echo "✓ packaging/aur/.SRCINFO (via makepkg)"
else
    echo "⚠  packaging/aur/.SRCINFO NOT regenerated — run on an Arch box:"
    echo "    (cd packaging/aur && makepkg --printsrcinfo > .SRCINFO)"
fi

# ---------- Scoop manifest ----------
scoop=packaging/scoop/authu.json
python3 - "$scoop" "$VERSION" "$SHA_WIN_X86_64" <<'PY'
import json, sys
path, ver, sha = sys.argv[1:]
with open(path) as f:
    m = json.load(f)
m['version'] = ver
m['architecture']['64bit']['url'] = (
    f"https://github.com/WangYihang/authu/releases/download/v{ver}/authu-x86_64-pc-windows-msvc.zip"
)
m['architecture']['64bit']['hash'] = sha
with open(path, 'w', encoding='utf-8') as f:
    json.dump(m, f, indent=4, ensure_ascii=False)
    f.write('\n')
PY
echo "✓ packaging/scoop/authu.json"

echo
echo "Bumped packaging templates to v${VERSION}. Diff:"
git --no-pager diff --stat packaging/
echo
echo "Next: sync each file into its publishing repo:"
echo "  - packaging/homebrew/authu.rb   → WangYihang/homebrew-tap:Formula/authu.rb"
echo "  - packaging/aur/PKGBUILD+.SRCINFO → ssh://aur@aur.archlinux.org/authu-bin.git"
echo "  - packaging/scoop/authu.json    → WangYihang/scoop-bucket:bucket/authu.json"
