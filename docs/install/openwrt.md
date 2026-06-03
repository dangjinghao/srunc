# OpenWrt

A LuCI app is published per release as a single `.ipk`. The package installs
the `srunc` CLI, an init script, the LuCI UI, and a `ubus` helper.

## 1. Download

From the [latest release](https://github.com/WangYihang/srunc/releases/latest),
grab the `.ipk` matching your router's arch:

| Router CPU | File |
|---|---|
| x86_64 | `luci-app-srunc_*_x86_64.ipk` |
| ARMv8 Cortex-A53 (most aarch64 SoCs, e.g. Allwinner H6, RK3328) | `luci-app-srunc_*_aarch64_cortex-a53.ipk` |
| ARMv8 Cortex-A72 (RPi 4, RK3399) | `luci-app-srunc_*_aarch64_cortex-a72.ipk` |
| ARMv8 generic | `luci-app-srunc_*_aarch64_generic.ipk` |
| ARMv7 Cortex-A7 + NEON-VFPv4 (MT7621, Allwinner H3) | `luci-app-srunc_*_arm_cortex-a7_neon-vfpv4.ipk` |

If unsure, check `opkg print-architecture` on the router.

## 2. Install

```sh
scp luci-app-srunc_*.ipk root@<router>:/tmp/
ssh root@<router>
opkg install /tmp/luci-app-srunc_*.ipk
```

The package depends on `luci-base`; OpenWrt 22.03+ already ships it.

## 3. Configure

Open LuCI → **Services → srunc**. Fill in username + password and hit save.
Logs live in the **Log** tab.
