# `packaging/openwrt/` — OpenWrt feed

This directory is structured as an [OpenWrt opkg feed](https://openwrt.org/docs/guide-developer/feeds): each subdirectory is one package.

```
packaging/openwrt/
└── luci-app-srunc/        ← the LuCI web UI for srunc (depends on /usr/sbin/srunc)
    ├── Makefile
    ├── root/              → installed at / on target
    └── htdocs/            → installed at /www on target
```

The `srunc` binary itself is **not** packaged as an opkg today — install it from the per-arch musl static tarball in [GitHub Releases](https://github.com/WangYihang/srunc/releases) (drop it at `/usr/sbin/srunc`). A future binary opkg may live here as a sibling `srunc/` directory.

## Building locally

From an OpenWrt buildroot with the `luci` feed available:

```sh
echo "src-link srunc $(realpath packaging/openwrt)" >> feeds.conf
./scripts/feeds update srunc
./scripts/feeds install -p srunc luci-app-srunc
make package/luci-app-srunc/compile V=s
# .ipk → bin/packages/<arch>/srunc/luci-app-srunc_*.ipk
```

## Building in CI

`.github/workflows/release.yml` builds the `.ipk` via [`openwrt/gh-action-sdk`](https://github.com/openwrt/gh-action-sdk) on every tag and attaches it to the GitHub Release.
