# srun

[![crates.io](https://img.shields.io/crates/v/srun.svg)](https://crates.io/crates/srun)
[![docs.rs](https://img.shields.io/docsrs/srun)](https://docs.rs/srun)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/gpl-3.0.html)

Sync HTTP client speaking the srun (深澜) campus-portal auth protocol, plus DNS-free captive-portal **auto-discovery**. Generic across srun deployments — Tsinghua's `auth4`/`auth6` are just the default fallback.

This crate is one piece of the [authu](https://github.com/wangyihang/authu) project. End users typically want the [`authu`](https://crates.io/crates/authu) CLI; this crate is for embedding srun auth into another Rust app.

## What's in here

- [`Client`](src/client.rs) — `ureq`-based HTTP client with `login`/`logout`/`status`
- [`Endpoints`](src/endpoints.rs) — per-host URL set (`from_host(host, insecure)`); deployment-agnostic
- [`discover`](src/discover.rs) — probe an HTTP target and parse the srun captive redirect for `{host, ac_id}` (no DNS needed)
- [`gateway::default_gateway_v4`](src/gateway.rs) — read `/proc/net/route` to find the default-route IP (Linux only; `None` elsewhere)
- [`tsinghua::detect_ac_id`](src/tsinghua.rs) — legacy DNS-based detection for the Tsinghua deployment

## Quick example

Add to your `Cargo.toml`: `srun-client = "0.1"` and `srun-core = "0.1"`.

```rust,no_run
use std::time::Duration;
use srun_client::{Client, Endpoints};
use srun_core::Credentials;

// Auto-discover host + ac_id from the default gateway (DNS-free):
let gw = srun_client::gateway::default_gateway_v4().expect("no default route");
let disc = srun_client::discover(&gw, Duration::from_secs(2))?;
let host = disc.host.unwrap_or_else(|| "auth4.tsinghua.edu.cn".into());
let ac_id = disc.ac_id.unwrap_or_else(|| "1".into());

let client = Client::new(Endpoints::from_host(&host, false), Duration::from_secs(2));
client.login(&Credentials { username: "alice", password: "hunter2" }, &ac_id, "")?;
let status = client.status(&ac_id)?;
assert!(status.online);
# Ok::<(), srun_client::Error>(())
```

## License

GPL-3.0-or-later. See [LICENSE](../../LICENSE).
