# luci-app-srunc — OpenWrt LuCI app for [srunc](../../README.md)

A modern (client-side JS) LuCI app for the `srunc` srun captive-portal auth
client, plus the supporting UCI schema, procd init, and rpcd ubus backend.

## What you get

- **Settings** (`Services › srunc › Settings`): a single username + password
  form over UCI, with a deployment-profile dropdown and optional overrides
  (host, ac_id, timeout, log level) plus keepalive tuning on the Advanced /
  Keepalive tabs.
- **Overview** (`Services › srunc › Overview`): live online/offline status as
  a status pill with user + IP, and three buttons — Login now / Logout /
  Restart service. Polls every 5 s via ubus.
- **Log** (`Services › srunc › Log`): recent srunc messages straight from the
  system log (`logread`), auto-refreshing every 5 s with a pause toggle. The
  *Log level* setting (Advanced) controls how much the daemon emits.
- **No manual ac_id / host config**: the `srunc` core auto-discovers both from
  the captive-portal redirect (DNS-free, by probing the default gateway).
- **Multiple accounts** (power users): the UI manages the `default` instance;
  add more `config srunc '<name>'` sections to `/etc/config/srunc` and each
  becomes its own procd-managed daemon.

## Files

| Path | Purpose |
|------|---------|
| `root/etc/config/srunc` | UCI schema with sane defaults |
| `root/etc/init.d/srunc` | procd service, multi-instance, renders UCI → tmpfs TOML (password not in argv) |
| `root/usr/libexec/rpcd/srunc` | rpcd ubus object (`status` / `log` / `login` / `logout` / `restart`) |
| `root/usr/share/rpcd/acl.d/luci-app-srunc.json` | LuCI session ACL grants |
| `root/usr/share/luci/menu.d/luci-app-srunc.json` | menu placement under Services |
| `htdocs/luci-static/resources/view/srunc/{overview,settings,log}.js` | client-side LuCI views |
| `Makefile` | OpenWrt buildroot recipe (`LUCI_PKGARCH:=all`) |

## Installing

You need **two** things on the target router:

1. The `srunc` binary at `/usr/sbin/srunc` — install from the project release
   tarball (per-arch musl static binary, no opkg required).
2. The `luci-app-srunc` opkg — built from this directory.

### Building the .ipk

From an OpenWrt buildroot (or SDK) with the `luci` feed available:

```sh
ln -s "$(realpath .)/packaging/openwrt" feeds/luci/applications/luci-app-srunc
./scripts/feeds update -i
./scripts/feeds install -p luci luci-app-srunc
make package/luci-app-srunc/compile V=s
# .ipk → bin/packages/<arch>/luci/luci-app-srunc_*.ipk
```

CI builds the .ipk automatically and attaches it to each GitHub Release
(see `.github/workflows/release.yml`).

### Installing on the router

```sh
opkg install /tmp/luci-app-srunc_*.ipk
# Then in LuCI: Services › srunc › Settings → set username + password → Save & Apply.
```

The first `Save & Apply` triggers `/etc/init.d/srunc reload`, which renders the
config, starts the procd instance, and the binary auto-discovers ac_id + host
from the gateway redirect — no manual fields needed.
