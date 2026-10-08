# Pollux Polyglot Native Bridge

![Banner](pollux-polyglot-native-bridge-banner.png)

![Version](https://img.shields.io/badge/version-0.1.0-blue)
![License](https://img.shields.io/badge/License-MIT-green)
[![CI](https://github.com/sxnnyside-project/pollux-polyglot-native-bridge/workflows/CI/badge.svg)](https://github.com/sxnnyside-project/pollux-polyglot-native-bridge/actions)

<p align="center">
  <strong>Zero Logic ✦ ABI Safe ✦ Direct FFI Boundary</strong><br>
  <em>The public Rust ABI/FFI boundary for Pollux — bridging polyglot runtimes to the deterministic core.</em>
</p>

<p align="center">
  <a href="#about">About</a> ✦
  <a href="#features">Features</a> ✦
  <a href="#installation">Installation</a> ✦
  <a href="#usage">Usage</a> ✦
  <a href="#architecture">Architecture</a> ✦
  <a href="#contributing">Contributing</a>
</p>

---

## About

**Pollux Polyglot Native Bridge** is the public Rust ABI/FFI boundary for [Pollux](https://sxnnysideproject.com) — the boundary a polyglot consumer crosses to reach a deterministic Pollux Core Decision, and nothing more.

Polyglot applications and external language runtimes require access to Pollux's deterministic authority evaluation without compiling Core from source or bundling duplicate policy logic. This crate resolves, links, and mediates communication with precompiled Pollux Core artifacts across the stable `pollux-abi/1` C interface.

It shuttles an Authority Manifest (YAML) in and an Evaluation Trace (JSON) out across the `extern "C"` boundary unmodified in either direction.

### Philosophy

> *"A bridge should carry data across the boundary without altering its meaning, inventing its own rules, or attempting to become the authority it serves."*

Pollux Polyglot Native Bridge is a CoreRed project, part of the Sxnnyside Project's Pollux Ecosystem.

---

## Features

- **Automated Artifact Resolution**: Downloads, verifies SHA-256 checksums, and links against precompiled Core distribution artifacts at build time.
- **Safe Handle Lifecycle**: `PolluxEngine` wraps opaque C handles with strict single-creation and single-destruction semantics via `Drop`.
- **Zero Policy Duplication**: Contains no internal evaluation rules, capability grants, or contracts; every decision originates directly from Pollux Core.
- **Typed SDK Surface**: Provides strongly-typed `Operation`, `Evaluation`, and `Outcome` structures without obscuring raw wire traces.
- **Cross-Platform ABI**: Mapped against five target triples with automatic macOS dynamic install-name relocation patching.

---

## Installation

### Prerequisites

- Rust (`1.93.0` or stable)
- Network access to GitHub releases for precompiled Core downloads
- macOS only: Xcode Command Line Tools (`install_name_tool`)

### From Source

```bash
git clone https://github.com/sxnnyside-project/pollux-polyglot-native-bridge.git
cd pollux-polyglot-native-bridge

just install
just build
```

---

## Usage

### Raw Wire Evaluation

```rust
use pollux_polyglot_native_bridge::{PolluxEngine, abi_version, core_version};

let engine = PolluxEngine::load(manifest_yaml)?;
let trace_json = engine.evaluate(operation_json)?;
// engine's underlying Core handle is destroyed automatically on drop.

println!("ABI: {} / Core: {}", abi_version(), core_version());
```

### Typed SDK Operations

```rust
use pollux_polyglot_native_bridge::{Operation, Outcome, PolluxEngine};

let engine = PolluxEngine::load(manifest_yaml)?;
let evaluation = engine.evaluate_operation(
    Operation::new("read", "filesystem", "./assets/report.txt")
)?;

match evaluation.outcome() {
    Some(Outcome::Allow) => {
        // proceed with operation
    }
    Some(Outcome::Deny) | None => {
        // refuse operation
    }
}
```

---

## Architecture

```text
pollux-polyglot-native-bridge/
├── build.rs             # Resolves, validates, and links the Core artifact
├── src/
│   ├── lib.rs           # Safe public API: PolluxEngine, BridgeError, versions
│   └── ffi.rs           # Raw extern "C" declarations (isolated unsafe boundary)
├── tests/
│   ├── integration.rs   # Exercises the real linked Core library
│   ├── conformance.rs   # Validates evaluations against the canonical test suite
│   └── sdk.rs           # Tests typed high-level SDK wrappers
└── ARTIFACT_LAYOUT.md   # Detailed Core artifact directory contract
```

For detailed binary and directory contracts, see [ARTIFACT_LAYOUT.md](ARTIFACT_LAYOUT.md).

---

## Supported Targets

| Target | Native Execution Verified |
|---|---|
| `aarch64-apple-darwin` |  yes — real public artifact linked and run |
| `x86_64-apple-darwin` | ❌ cross-compiles (runner verification pending) |
| `x86_64-unknown-linux-gnu` | ❌ cross-compiles (runner verification pending) |
| `aarch64-unknown-linux-gnu` | ❌ cross-compiles (runner verification pending) |
| `x86_64-pc-windows-msvc` | ❌ cross-compiles (runner verification pending) |

---

## Contributing

Contributions are accepted. See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

Before contributing, read the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

<p align="center">
  <strong>Pollux Polyglot Native Bridge</strong> — A CoreRed Project<br>
  <em>&copy; 2026 Sxnnyside Project</em>
</p>
