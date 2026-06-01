# OpenWrt feed package — luci-app-authu.
#
# Noarch (PKGARCH=all) LuCI app. Expects the `authu` binary at /usr/sbin/authu
# (install separately from the project release tarball; per-arch musl static).
#
# Built by CI via openwrt/gh-action-sdk; locally you can:
#
#   echo "src-link authu $(realpath packaging/openwrt)" >> feeds.conf
#   ./scripts/feeds update authu && ./scripts/feeds install -p authu luci-app-authu
#   make package/luci-app-authu/compile V=s

include $(TOPDIR)/rules.mk

PKG_NAME:=luci-app-authu
PKG_VERSION:=0.2.0
PKG_RELEASE:=1
PKG_LICENSE:=GPL-3.0-or-later
PKG_MAINTAINER:=Yihang Wang <wangyihanger@gmail.com>

LUCI_TITLE:=LuCI support for authu (srun campus-portal auth)
LUCI_DESCRIPTION:=\
  Web UI for the generic srun (深澜) captive-portal auth client `authu`. \
  Auto-discovers the portal host and ac_id from the captive redirect — \
  most users only need to set username + password. Multi-instance via UCI. \
  Requires /usr/sbin/authu (install from the project release tarball).
LUCI_DEPENDS:=
LUCI_PKGARCH:=all

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
	$(INSTALL_DIR)  $(1)/usr/share/rpcd/acl.d
	$(INSTALL_DIR)  $(1)/usr/share/luci/menu.d
	$(INSTALL_DIR)  $(1)/www/luci-static/resources/view/authu
	$(INSTALL_CONF) ./root/etc/config/authu                              $(1)/etc/config/authu
	$(INSTALL_BIN)  ./root/etc/init.d/authu                              $(1)/etc/init.d/authu
	$(INSTALL_BIN)  ./root/usr/libexec/rpcd/authu                        $(1)/usr/libexec/rpcd/authu
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
