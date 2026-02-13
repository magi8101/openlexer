# Installation

## Prerequisites

- Rust toolchain (1.70 or later)
- Cargo package manager

## Build from Source

```bash
git clone https://github.com/magi8101/openlexer.git
cd openlexer
cargo build --release
```

The binary will be at `target/release/openlexer` (or `openlexer.exe` on Windows).

## Install via Cargo

```bash
cargo install --path .
```

This installs `openlexer` to `~/.cargo/bin/`.

## Verify Installation

```bash
openlexer --version
openlexer --help
```

## WebAssembly Build

To build for web browsers:

```bash
cargo install wasm-pack
wasm-pack build --target web
```

The output will be in the `pkg/` directory.

## Platform Notes

### Windows

No additional dependencies required. The MSVC toolchain is recommended.

### Linux

Ensure `build-essential` or equivalent is installed for linking.

### macOS

Xcode command line tools must be installed:

```bash
xcode-select --install
```
