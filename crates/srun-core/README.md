# srun-core

[![crates.io](https://img.shields.io/crates/v/srun-core.svg)](https://crates.io/crates/srun-core)
[![docs.rs](https://img.shields.io/docsrs/srun-core)](https://docs.rs/srun-core)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/gpl-3.0.html)

Pure, `no_std`-friendly **crypto core** of the srun (深澜) campus-portal auth protocol. Zero I/O, zero global state, byte-for-byte equivalent to the upstream Go reference (golden test vectors pinned).

This crate is one piece of the [authu](https://github.com/wangyihang/authu) project — most users want the [`authu`](https://crates.io/crates/authu) CLI (or the [`srun`](https://crates.io/crates/srun) I/O shell) rather than this raw protocol crate.

## What's in here

- [`XEncode`](src/xencode.rs) — srun's XXTEA-variant block cipher
- [`QuirkBase64Encode`](src/base64.rs) — the custom-alphabet base64 used in srun's login payload
- [`MD5`/`SHA1`](src/digest.rs) helpers (RustCrypto under the hood)
- [`build_portal_params`](src/params.rs) — assemble the login/logout HTTP query
- [`PortalError`](src/error.rs) — typed error codes (`E2553` wrong password, `E2620` already online, …)

## License

GPL-3.0-or-later. See [LICENSE](../../LICENSE).
