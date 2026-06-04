# OpenWrt feed package — luci-app-srunc.
#
# **Arch-specific** (not noarch): bundles the prebuilt static-musl `srunc`
# binary alongside the LuCI assets, so `opkg install luci-app-srunc_*.ipk`
# installs everything in one shot. The binary is fetched from the project's
# GitHub Release per-arch tarball and placed at `./files/srunc` before this
# Makefile runs — see `.github/workflows/release.yml` for the CI plumbing.
#
# Locally (rare; CI normally drives this):
#
#   # 1. Drop the prebuilt static-musl binary for your target into ./files/
#   curl -L https://github.com/WangYihang/srunc/releases/latest/download/srunc-x86_64-unknown-linux-musl.tar.gz \
#     | tar -xz -C /tmp/ && cp /tmp/srunc-x86_64-unknown-linux-musl/srunc ./files/srunc
#   # 2. Build with the OpenWrt SDK
#   echo "src-link srunc $(realpath packaging/openwrt)" >> feeds.conf
#   ./scripts/feeds update srunc && ./scripts/feeds install -p srunc luci-app-srunc
#   make package/luci-app-srunc/compile V=s

include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-srunc
PKG_VERSION:=0.4.3
PKG_RELEASE:=1
PKG_LICENSE:=MIT
PKG_MAINTAINER:=Yihang Wang <wangyihanger@gmail.com>

LUCI_TITLE:=srunc (srun campus-portal auth) + LuCI web UI
LUCI_DESCRIPTION:=\
  Generic srun (深澜) captive-portal auth client with a modern LuCI web UI \
  under Services › srunc. Auto-discovers the portal host and ac_id from \
  the captive redirect — most users only need to set username + password. \
  Multi-instance via UCI. Bundles the static-musl `srunc` binary; no \
  separate download needed.
LUCI_DEPENDS:=
# luci.mk defaults LUCI_PKGARCH to "all" unless `src/Makefile` exists.
# Setting it to empty here keeps it from being defaulted, leaving PKGARCH
# unset → luci.mk falls through and BuildPackage picks the SDK's target
# arch, so we produce one .ipk per architecture (each bundling its own
# musl binary) instead of a single noarch package.
LUCI_PKGARCH:=

# Absolute path to luci.mk so the package builds from any feed location
# (the standard `include ../../luci.mk` only resolves when located inside
# the luci feed at applications/<name>/Makefile).
include $(TOPDIR)/feeds/luci/luci.mk

define Package/luci-app-srunc/conffiles
/etc/config/srunc
endef

# No custom Package/.../install rule: luci.mk unconditionally redefines it
# (the user-defined version is silently overwritten — that's why we need to
# stay on the convention). Its default copies root/* → / and
# htdocs/* → /www/ preserving permissions, which covers every file we ship,
# including the per-arch `srunc` binary that CI stages at root/usr/sbin/srunc.

define Package/luci-app-srunc/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] && exit 0
/etc/init.d/srunc enable >/dev/null 2>&1 || true
/etc/init.d/rpcd restart >/dev/null 2>&1 || true
exit 0
endef

# call BuildPackage - OpenWrt buildroot signature
