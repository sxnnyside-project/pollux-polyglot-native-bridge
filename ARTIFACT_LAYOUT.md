# Pollux Core artifact contract

This crate never builds Pollux Core from source and never vendors Core
source into this repository. It links against exactly one precompiled
artifact, identified by three coordinates:

- **Core version** — e.g. `0.1.0`, matching `pollux-authority`'s
  `Cargo.toml` version in the private `pollux` (Core) repository.
- **Target triple** — e.g. `aarch64-apple-darwin`, `x86_64-unknown-linux-gnu`.
- **ABI version** — e.g. `pollux-abi/1`, `pollux_ffi::ABI_VERSION` /
  `pollux_abi_version()`. Bumped only when the `extern "C"` surface itself
  changes shape, independent of the Core version.

## Expected directory shape

The public distribution repository (`pollux-core`, the artifact host —
not this repository, and not the private Core source repository) is
expected to publish one directory per `(core version, target triple)`
pair:

```text
pollux-core-<core-version>-<target-triple>/
├── abi                     # plain text file: the ABI version this
│                            # artifact implements, e.g. "pollux-abi/1"
├── include/
│   └── pollux.h            # cbindgen-generated header, reproducible
│                            # byte-for-byte from pollux-ffi's source
├── lib/
│   └── libpollux_ffi.{dylib,so} / pollux_ffi.dll
└── SHA256SUMS               # checksums for lib/ and include/ contents
```

Example: `pollux-core-0.1.0-aarch64-apple-darwin/`.

## Core version this crate consumes

Pinned in `build.rs` as `CORE_VERSION = "0.1.0"`. Bumped deliberately, in
its own change — never resolved from "latest".

## How this crate obtains it

This is a binary/ABI dependency on the published `pollux-core` release —
not a Cargo source dependency, and not a build against a local or
private Core checkout. `build.rs` always fetches
`pollux-core-<CORE_VERSION>-<target>.<tar.gz|zip>` from
`sxnnyside-project/pollux-core`'s `v<CORE_VERSION>` GitHub release
(https://github.com/sxnnyside-project/pollux-core/releases/tag/v0.1.0),
into `target/pollux-core-dist-cache/`. A clean checkout with no local
setup builds successfully this way; there is no local override or
vendored-binary path in the normal architecture.

The downloaded artifact is validated before linking: `include/pollux.h`
and the target's library file must both be present, and if a
`SHA256SUMS` file is present, every listed file's SHA-256 is verified
against it. Any failure (download failure, missing file, checksum
mismatch, unsupported target) panics the build with a specific,
actionable message — never a silent fallback to a different artifact.

## Supported targets

`x86_64-unknown-linux-gnu`, `aarch64-unknown-linux-gnu`,
`x86_64-apple-darwin`, `aarch64-apple-darwin`, `x86_64-pc-windows-msvc` —
matching Pollux Core's own release matrix exactly. An unsupported target
fails the build naming the requested target, the supported list, and the
artifact name that would have been expected.

## Running the tests

`cargo test` exercises the actual linked Core library — no ABI mocking.
`build.rs` downloads the artifact automatically; a working network
connection to GitHub is the only requirement.

## Compatibility rule

This crate's own `abi_version()` wrapper always reflects whatever it is
actually linked against — it does not hardcode an expected value. A
version-negotiation step (comparing the linked artifact's
`pollux_abi_version()` against the ABI version this crate's bindings in
`src/ffi.rs` were written against) will be introduced once a second
ABI version exists to negotiate against.
