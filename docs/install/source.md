# From source

Requires Rust 1.74 or newer (`rustup` recommended).

## From crates.io

```sh
cargo install srunc
```

Drops `srunc` in `~/.cargo/bin/`.

## From a local clone

```sh
git clone https://github.com/WangYihang/srunc
cd srunc
cargo build --release          # binary at target/release/srunc
cargo install --path .         # or install into ~/.cargo/bin/
```

## Library-only (no CLI deps)

If you're embedding `srunc` in a Rust app and don't need argv parsing,
terminal styling, or the TOML config loader:

```sh
cargo add srunc --no-default-features
```

This drops `clap`, `anstream`, `env_logger`, `rpassword`, `ctrlc`,
`basic-toml`, and `anyhow`.
