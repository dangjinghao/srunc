# srun-client

[![crates.io](https://img.shields.io/crates/v/srun-client.svg)](https://crates.io/crates/srun-client)
[![docs.rs](https://img.shields.io/docsrs/srun-client)](https://docs.rs/srun-client)
[![License: GPL-3.0-or-later](https://img.shields.io/badge/license-GPL--3.0--or--later-blue.svg)](https://www.gnu.org/licenses/gpl-3.0.html)

Sync HTTP client speaking the srun (深澜) captive-portal auth protocol, plus DNS-free captive-redirect **auto-discovery** so it works against any srun deployment without hand-fed `host` / `ac_id`.

One piece of the [srunc](https://github.com/WangYihang/srunc) project; end users typically want the [`srunc`](https://crates.io/crates/srunc) CLI. This crate is for embedding srun auth into another Rust app.

## What's in here

- [`Client`](src/client.rs) — `login` / `logout` / `status`
- [`Endpoints::from_host`](src/endpoints.rs) — build the per-host URL set
- [`discover`](src/discover.rs) — probe an HTTP target, parse the srun captive redirect for `{host, ac_id}`
- [`gateway::default_gateway_v4`](src/gateway.rs) — read `/proc/net/route` for the default-route IP (Linux only)
- [`profiles`](src/profiles.rs) — built-in `(host, ac_id)` presets for known srun deployments

## Quick example

```rust,no_run
use std::time::Duration;
use srun_client::{Client, Endpoints, discover, gateway};
use srun_core::Credentials;

let gw = gateway::default_gateway_v4().expect("no default route");
let disc = discover(&gw, Duration::from_secs(2))?;
let host = disc.host.expect("captive redirect carries host");
let ac_id = disc.ac_id.expect("captive redirect carries ac_id");

let client = Client::new(Endpoints::from_host(&host, false), Duration::from_secs(2));
client.login(&Credentials { username: "alice", password: "hunter2" }, &ac_id, "")?;
assert!(client.status(&ac_id)?.online);
# Ok::<(), srun_client::Error>(())
```

## License

GPL-3.0-or-later. See [LICENSE](../../LICENSE).
