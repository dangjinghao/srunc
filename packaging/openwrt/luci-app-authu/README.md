# luci-app-authu — OpenWrt LuCI app for [authu](../../README.md)

A modern (client-side JS) LuCI app for the `authu` srun captive-portal auth
client, plus the supporting UCI schema, procd init, and rpcd ubus backend.

## What you get

- **Settings** (`Services › authu › Settings`): username + password form over
  UCI, with optional overrides (host, ac_id, ipv6, timeout, keepalive) on an
  Advanced tab. Multi-instance: add as many `config authu '<name>'` sections
  as you like; each becomes its own procd-managed daemon.
- **Overview** (`Services › authu › Overview`): live online/offline status,
  user + IP, and three buttons — Login now / Logout / Restart service. Polls
  every 5 s via ubus.
- **No manual ac_id / host config**: the `authu` core auto-discovers both from
  the captive-portal redirect (DNS-free, by probing the default gateway).

## Files

| Path | Purpose |
|------|---------|
| `root/etc/config/authu` | UCI schema with sane defaults |
| `root/etc/init.d/authu` | procd service, multi-instance, renders UCI → tmpfs TOML (password not in argv) |
| `root/usr/libexec/rpcd/authu` | rpcd ubus object (`status` / `login` / `logout` / `restart`) |
| `root/usr/share/rpcd/acl.d/luci-app-authu.json` | LuCI session ACL grants |
| `root/usr/share/luci/menu.d/luci-app-authu.json` | menu placement under Services |
| `htdocs/luci-static/resources/view/authu/{overview,settings}.js` | client-side LuCI views |
| `Makefile` | OpenWrt buildroot recipe (`LUCI_PKGARCH:=all`) |

## Installing

You need **two** things on the target router:

1. The `authu` binary at `/usr/sbin/authu` — install from the project release
   tarball (per-arch musl static binary, no opkg required).
2. The `luci-app-authu` opkg — built from this directory.

### Building the .ipk

From an OpenWrt buildroot (or SDK) with the `luci` feed available:

```sh
ln -s "$(realpath .)/packaging/openwrt" feeds/luci/applications/luci-app-authu
./scripts/feeds update -i
./scripts/feeds install -p luci luci-app-authu
make package/luci-app-authu/compile V=s
# .ipk → bin/packages/<arch>/luci/luci-app-authu_*.ipk
```

CI builds the .ipk automatically and attaches it to each GitHub Release
(see `.github/workflows/release.yml`).

### Installing on the router

```sh
opkg install /tmp/luci-app-authu_*.ipk
# Then in LuCI: Services › authu › Settings → set username + password → Save & Apply.
```

The first `Save & Apply` triggers `/etc/init.d/authu reload`, which renders the
config, starts the procd instance, and the binary auto-discovers ac_id + host
from the gateway redirect — no manual fields needed.
