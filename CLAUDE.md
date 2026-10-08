# Pollux Polyglot Native Bridge — Contributor & AI Guidelines

## What this repository is

`pollux-polyglot-native-bridge` is the public Rust ABI/FFI boundary for Pollux Core (`pollux-abi/1`). It is the minimal bridge layer through which polyglot consumers interact with a precompiled Pollux Core execution authority engine.

It defines:
- A safe Rust lifecycle wrapper (`PolluxEngine`) around the opaque Core C ABI handle.
- Typed `Operation`, `Evaluation`, and `Outcome` views over raw wire protocol JSON.
- Automated validation, linking, and macOS install name patching of precompiled release artifacts.

## What this repository is NOT

- **Not an authorization engine:** It contains no rules, grants, capabilities, contracts, or policies.
- **Not an enforcement broker:** It passes queries to Core and returns decisions. Kernel mediation or process sandboxing belongs to external integration layers.

## Standards and Quality Gates

All development commands are governed by `Justfile`:

```bash
just install       # Fetch dependencies
just check         # Full gate: format + lint + check + test + cargo deny
just test          # Run integration, conformance, and SDK tests
```

- Code must pass `cargo fmt --check`.
- `clippy` is enforced with `-D warnings` in pedantic mode.
- Dependencies and licenses must satisfy `cargo deny check`.
- Zero `.unwrap()` or `.expect()` in non-test production code.
