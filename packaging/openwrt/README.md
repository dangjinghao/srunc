# `packaging/openwrt/` — OpenWrt feed

This directory is structured as an [OpenWrt opkg feed](https://openwrt.org/docs/guide-developer/feeds): each subdirectory is one package.

```
packaging/openwrt/
└── luci-app-authu/        ← the LuCI web UI for authu (depends on /usr/sbin/authu)
    ├── Makefile
    ├── root/              → installed at / on target
    └── htdocs/            → installed at /www on target
```

The `authu` binary itself is **not** packaged as an opkg today — install it from the per-arch musl static tarball in [GitHub Releases](https://github.com/wangyihang/authu/releases) (drop it at `/usr/sbin/authu`). A future binary opkg may live here as a sibling `authu/` directory.

## Building locally

From an OpenWrt buildroot with the `luci` feed available:

```sh
echo "src-link authu $(realpath packaging/openwrt)" >> feeds.conf
./scripts/feeds update authu
./scripts/feeds install -p authu luci-app-authu
make package/luci-app-authu/compile V=s
# .ipk → bin/packages/<arch>/authu/luci-app-authu_*.ipk
```

## Building in CI

`.github/workflows/release.yml` builds the `.ipk` via [`openwrt/gh-action-sdk`](https://github.com/openwrt/gh-action-sdk) on every tag and attaches it to the GitHub Release.
