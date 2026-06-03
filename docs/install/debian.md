# Debian / Ubuntu

A `.deb` is published per release for `amd64` and `arm64`. It installs:

- `/usr/bin/srunc` — the CLI
- `/lib/systemd/system/srunc.service` — one-shot system unit
- `/lib/systemd/system/srunc@.service` — templated unit (one instance per config)
- `/usr/lib/systemd/user/srunc.service` — user-session unit
- `/usr/share/doc/srunc/config.example.toml` — annotated config template

## Install

From the [latest release](https://github.com/WangYihang/srunc/releases/latest),
download the `.deb` matching your arch (`amd64` or `arm64`), then:

```sh
sudo apt install ./srunc_*.deb        # or: sudo dpkg -i srunc_*.deb
```

## Run as a service

The shipped unit looks for `/etc/srunc/config.toml`:

```sh
sudo mkdir -p /etc/srunc
sudo cp /usr/share/doc/srunc/config.example.toml /etc/srunc/config.toml
sudoedit /etc/srunc/config.toml       # set username + password
sudo systemctl enable --now srunc
journalctl -u srunc -f
```

For multiple sessions use the templated unit:
`systemctl enable --now srunc@<name>` reads `/etc/srunc/<name>.toml`.
