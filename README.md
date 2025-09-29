# CAN viewer

#### Developed by Kristian Sloth Lauszus, 2025
_________
[![Crates.io](https://img.shields.io/crates/v/can-viewer-rs.svg)](https://crates.io/crates/can-viewer-rs)
[![CI](https://github.com/Lauszus/can-viewer-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/Lauszus/can-viewer-rs/actions/workflows/ci.yml)

A terminal-based CAN bus viewer for Linux SocketCAN written in Rust.

It is a port of my original [Python CAN viewer](https://github.com/Lauszus/python_can_viewer), which is integrated into [python-can](https://github.com/hardbyte/python-can).

## Usage

To test this application you can either install it via `cargo`:

```bash
cargo install can-viewer-rs
```

Or run pre-compiled binaries using [uvx](https://github.com/astral-sh/uv):

```bash
uvx can-viewer-rs --channel can0
```

Some pre-compiled binaries are also available on the [Releases](https://github.com/Lauszus/can-viewer-rs/releases) page.

## Development

### Run

```bash
make run
```

### Build

```bash
make build-debug
```

```bash
make build-release
```

### Wheel

```bash
make wheel
```
