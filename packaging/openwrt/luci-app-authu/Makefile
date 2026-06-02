# OpenWrt feed package — luci-app-authu.
#
# **Arch-specific** (not noarch): bundles the prebuilt static-musl `authu`
# binary alongside the LuCI assets, so `opkg install luci-app-authu_*.ipk`
# installs everything in one shot. The binary is fetched from the project's
# GitHub Release per-arch tarball and placed at `./files/authu` before this
# Makefile runs — see `.github/workflows/release.yml` for the CI plumbing.
#
# Locally (rare; CI normally drives this):
#
#   # 1. Drop the prebuilt static-musl binary for your target into ./files/
#   curl -L https://github.com/WangYihang/authu/releases/latest/download/authu-x86_64-unknown-linux-musl.tar.gz \
#     | tar -xz -C /tmp/ && cp /tmp/authu-x86_64-unknown-linux-musl/authu ./files/authu
#   # 2. Build with the OpenWrt SDK
#   echo "src-link authu $(realpath packaging/openwrt)" >> feeds.conf
#   ./scripts/feeds update authu && ./scripts/feeds install -p authu luci-app-authu
#   make package/luci-app-authu/compile V=s

include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-authu
PKG_VERSION:=0.2.4
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0-or-later
PKG_MAINTAINER:=Yihang Wang <wangyihanger@gmail.com>

LUCI_TITLE:=authu (srun campus-portal auth) + LuCI web UI
LUCI_DESCRIPTION:=\
  Generic srun (深澜) captive-portal auth client with a modern LuCI web UI \
  under Services › authu. Auto-discovers the portal host and ac_id from \
  the captive redirect — most users only need to set username + password. \
  Multi-instance via UCI. Bundles the static-musl `authu` binary; no \
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

define Package/luci-app-authu/conffiles
/etc/config/authu
endef

define Package/luci-app-authu/install
	$(INSTALL_DIR)  $(1)/etc/config
	$(INSTALL_DIR)  $(1)/etc/init.d
	$(INSTALL_DIR)  $(1)/usr/libexec/rpcd
	$(INSTALL_DIR)  $(1)/usr/sbin
	$(INSTALL_DIR)  $(1)/usr/share/rpcd/acl.d
	$(INSTALL_DIR)  $(1)/usr/share/luci/menu.d
	$(INSTALL_DIR)  $(1)/www/luci-static/resources/view/authu
	$(INSTALL_CONF) ./root/etc/config/authu                              $(1)/etc/config/authu
	$(INSTALL_BIN)  ./root/etc/init.d/authu                              $(1)/etc/init.d/authu
	$(INSTALL_BIN)  ./root/usr/libexec/rpcd/authu                        $(1)/usr/libexec/rpcd/authu
	$(INSTALL_BIN)  ./files/authu                                        $(1)/usr/sbin/authu
	$(INSTALL_DATA) ./root/usr/share/rpcd/acl.d/luci-app-authu.json      $(1)/usr/share/rpcd/acl.d/luci-app-authu.json
	$(INSTALL_DATA) ./root/usr/share/luci/menu.d/luci-app-authu.json     $(1)/usr/share/luci/menu.d/luci-app-authu.json
	$(INSTALL_DATA) ./htdocs/luci-static/resources/view/authu/*.js       $(1)/www/luci-static/resources/view/authu/
endef

define Package/luci-app-authu/postinst
#!/bin/sh
[ -n "$${IPKG_INSTROOT}" ] && exit 0
/etc/init.d/authu enable >/dev/null 2>&1 || true
/etc/init.d/rpcd restart >/dev/null 2>&1 || true
exit 0
endef

# call BuildPackage - OpenWrt buildroot signature
