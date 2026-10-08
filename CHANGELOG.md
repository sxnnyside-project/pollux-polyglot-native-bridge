# Changelog

All notable changes to **pollux-polyglot-native-bridge** are documented here.

This project follows [Keep a Changelog](https://keepachangelog.com/en/1.1.0/)
and [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [Unreleased]

### Added

---

## [0.1.0] — 2026-10-08

### Added

- Initial implementation of the public Rust ABI/FFI boundary for Pollux Core (`pollux-abi/1`).
- Safe `PolluxEngine` lifecycle abstraction managing precompiled Core handle allocation and destruction.
- Typed `Operation`, `Evaluation`, and `Outcome` wrapper over raw wire JSON.
- Automated precompiled Core artifact resolution, SHA-256 verification, and link configuration in `build.rs`.
- Comprehensive integration and conformance test suites verifying evaluations against official test vectors.

---

[Unreleased]: https://github.com/sxnnyside-project/pollux-polyglot-native-bridge/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/sxnnyside-project/pollux-polyglot-native-bridge/releases/tag/v0.1.0
