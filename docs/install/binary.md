# Prebuilt binary (macOS / Windows / other Linux)

Each release ships static binaries for:

- `x86_64-unknown-linux-gnu` / `x86_64-unknown-linux-musl`
- `aarch64-unknown-linux-gnu` / `aarch64-unknown-linux-musl`
- `armv7-unknown-linux-musleabihf`, `arm-unknown-linux-musleabi`
- `x86_64-apple-darwin`, `aarch64-apple-darwin`
- `x86_64-pc-windows-msvc`

## Option A: cargo binstall (recommended)

```sh
cargo binstall srunc
```

Fetches the correct tarball for your host and drops `srunc` in
`~/.cargo/bin/`.

## Option B: download by hand

From the [latest release](https://github.com/WangYihang/srunc/releases/latest):

```sh
curl -LO https://github.com/WangYihang/srunc/releases/latest/download/srunc-x86_64-unknown-linux-musl.tar.gz
tar xzf srunc-x86_64-unknown-linux-musl.tar.gz
sudo install -m 755 srunc-x86_64-unknown-linux-musl/srunc /usr/local/bin/
```

Substitute the target triple to match your host. Windows builds ship as a
`.zip` containing `srunc.exe`.

## Verify

```sh
sha256sum -c SHA256SUMS         # download SHA256SUMS from the same release
srunc --version
```
