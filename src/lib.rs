//! `pollux-polyglot-native-bridge` — the public Rust FFI/ABI bridge
//! between Pollux Core (private) and external consumers.
//!
//! # Responsibility
//!
//! ```text
//! external consumer -> Native Bridge -> Pollux Core ABI -> Pollux Core
//! ```
//!
//! This crate holds an opaque engine handle and shuttles opaque byte
//! strings (an Authority Manifest as YAML, an Operation and its resulting
//! Evaluation Trace as `pollux-protocol/1` JSON) across the ABI boundary
//! `pollux-ffi` defines. It does not parse, interpret, or validate the
//! *content* of those strings — Pollux Core does that, and only Pollux
//! Core.
//!
//! This crate explicitly does **not** implement: Capability semantics,
//! Resource normalization, Authority evaluation, Contract evaluation,
//! Decision logic, or any fallback authorization logic. Every Decision
//! returned by [`PolluxEngine::evaluate`] was produced by Pollux Core;
//! this crate cannot compute one on its own even by accident, because it
//! has no `Capability`, `Resource`, `Grant`, or `Decision` type of its
//! own — only a pointer and two string buffers.

mod ffi;
mod operation;

use std::ffi::CStr;
use std::fmt;
use std::os::raw::c_char;
use std::ptr;
use std::str::Utf8Error;

pub use ffi::PolluxStatus;
pub use operation::{Evaluation, Operation, Outcome};

/// Everything that can go wrong crossing the ABI from this side.
///
/// Note what is *not* here: there is no "manifest invalid" or "operation
/// denied" variant of this crate's own. Those are Core's answers
/// ([`PolluxStatus::ManifestError`], [`PolluxStatus::OperationError`], or
/// a `deny` outcome inside a successfully returned trace) — this crate
/// only reports whether the *call itself* crossed the ABI cleanly.
#[derive(Debug)]
pub enum BridgeError {
    /// Core rejected the call; carries the raw ABI status it returned.
    Core(PolluxStatus),
    /// A string Core returned across the ABI was not valid UTF-8.
    InvalidUtf8(Utf8Error),
    /// The linked Pollux Core artifact reports an ABI version this
    /// crate's `src/ffi.rs` was not written against. Checked once, before
    /// any other ABI call, so a mismatched artifact fails clearly instead
    /// of producing undefined behavior through a silently incompatible
    /// `extern "C"` surface.
    IncompatibleAbi {
        linked: String,
        expected: &'static str,
    },
}

/// The ABI version `src/ffi.rs`'s declarations were hand-written against.
/// Compared against the linked artifact's own `pollux_abi_version()` by
/// [`PolluxEngine::load`] before any other ABI call is made.
const EXPECTED_ABI_VERSION: &str = "pollux-abi/1";

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Core(status) => write!(f, "Pollux Core rejected the call: {status:?}"),
            Self::InvalidUtf8(source) => {
                write!(f, "Pollux Core returned non-UTF-8 data: {source}")
            }
            Self::IncompatibleAbi { linked, expected } => write!(
                f,
                "linked Pollux Core artifact implements ABI `{linked}`, but this build of \
                 pollux-polyglot-native-bridge was written against `{expected}`"
            ),
        }
    }
}

impl std::error::Error for BridgeError {}

/// The ABI version this build of the bridge was compiled against, as
/// reported by the linked Pollux Core artifact.
///
/// # Panics
///
/// Panics if Core's string is not valid UTF-8. Core's own ABI guarantees
/// this never happens for its static version strings; a panic here would
/// mean the linked artifact does not actually implement `pollux-abi/1`.
pub fn abi_version() -> &'static str {
    // SAFETY: `pollux_abi_version` returns a pointer to a `'static`,
    // NUL-terminated string per the ABI contract; this crate never frees
    // it, matching that contract.
    unsafe { CStr::from_ptr(ffi::pollux_abi_version()) }
        .to_str()
        .expect("Pollux Core ABI version string must be valid UTF-8")
}

/// The Core evaluation-model version reported by the linked Pollux Core
/// artifact.
///
/// # Panics
///
/// See [`abi_version`] — the same guarantee applies here.
pub fn core_version() -> &'static str {
    // SAFETY: see `abi_version`.
    unsafe { CStr::from_ptr(ffi::pollux_core_version()) }
        .to_str()
        .expect("Pollux Core version string must be valid UTF-8")
}

/// An owned handle to a Pollux Core `AuthorityEngine`, loaded across the
/// ABI. Destroys the underlying Core engine on [`Drop`].
pub struct PolluxEngine {
    handle: ptr::NonNull<ffi::PolluxEngineHandle>,
}

// SAFETY: the linked `AuthorityEngine::evaluate` is documented by Core as
// pure with respect to `self` — concurrent calls on any thread never
// mutate engine state, so nothing races. Ownership (construction and
// `Drop`) is not shared across the `Send` boundary in any way that would
// double-free: exactly one `PolluxEngine` owns `handle`.
unsafe impl Send for PolluxEngine {}
unsafe impl Sync for PolluxEngine {}

/// Checks the linked Pollux Core artifact's ABI version against the one
/// `src/ffi.rs` was written against. [`PolluxEngine::load`] calls this
/// before any other ABI call; exposed separately so a caller can check
/// compatibility up front, without needing a Manifest yet.
pub fn check_abi_compatibility() -> Result<(), BridgeError> {
    let linked = abi_version();
    if linked != EXPECTED_ABI_VERSION {
        return Err(BridgeError::IncompatibleAbi {
            linked: linked.to_string(),
            expected: EXPECTED_ABI_VERSION,
        });
    }
    Ok(())
}

impl PolluxEngine {
    /// Loads a Pollux Core engine from Authority Manifest YAML bytes,
    /// crossing the ABI exactly once.
    pub fn load(manifest_yaml: &str) -> Result<Self, BridgeError> {
        check_abi_compatibility()?;

        let mut raw: *mut ffi::PolluxEngineHandle = ptr::null_mut();
        // SAFETY: `manifest_yaml` is a valid, readable `&str` for its
        // whole length; `&mut raw` is a valid, writable out-pointer.
        let status = unsafe {
            ffi::pollux_engine_create(manifest_yaml.as_ptr(), manifest_yaml.len(), &mut raw)
        };
        match status {
            PolluxStatus::Ok => {
                let handle = ptr::NonNull::new(raw)
                    .expect("Core must set a non-null handle on PolluxStatus::Ok");
                Ok(Self { handle })
            }
            other => Err(BridgeError::Core(other)),
        }
    }

    /// Evaluates one Operation (as `pollux-protocol/1` `OperationWire`
    /// JSON — this crate does not construct or validate that shape
    /// itself, it only forwards the caller's bytes) and returns Core's
    /// `EvaluationTraceWire` JSON verbatim.
    ///
    /// The returned string is exactly what Pollux Core produced; this
    /// crate does not inspect its `decision.outcome` field or otherwise
    /// interpret the result.
    pub fn evaluate(&self, operation_json: &str) -> Result<String, BridgeError> {
        let mut out: *mut c_char = ptr::null_mut();
        // SAFETY: `self.handle` is a live handle for the lifetime of
        // `self` (destroyed only in `Drop`); `operation_json` is a valid
        // `&str` for its whole length; `&mut out` is a valid out-pointer.
        let status = unsafe {
            ffi::pollux_engine_evaluate(
                self.handle.as_ptr(),
                operation_json.as_ptr(),
                operation_json.len(),
                &mut out,
            )
        };

        if status != PolluxStatus::Ok {
            return Err(BridgeError::Core(status));
        }

        // SAFETY: `out` was just written by `pollux_engine_evaluate` on
        // `PolluxStatus::Ok`, per its documented contract.
        let result = unsafe { CStr::from_ptr(out) }
            .to_str()
            .map(str::to_owned)
            .map_err(BridgeError::InvalidUtf8);

        // SAFETY: `out` is freed exactly once, after being read, matching
        // `pollux_string_free`'s contract.
        unsafe { ffi::pollux_string_free(out) };

        result
    }

    /// The SDK-facing counterpart of [`Self::evaluate`]: takes a typed
    /// [`Operation`] instead of a hand-assembled JSON string, and
    /// returns an [`Evaluation`] instead of a raw `String` — so a native
    /// SDK consumer built on this crate never has to construct or parse
    /// `pollux-protocol/1` wire JSON directly. [`Operation::to_wire_json`]
    /// produces exactly the same bytes a caller of [`Self::evaluate`]
    /// would have written by hand; this method adds no new ABI call,
    /// wire shape, or interpretation of Core's answer beyond what
    /// [`Evaluation::outcome`] already documents.
    pub fn evaluate_operation(&self, operation: Operation<'_>) -> Result<Evaluation, BridgeError> {
        self.evaluate(&operation.to_wire_json())
            .map(Evaluation::from_trace_json)
    }
}

impl Drop for PolluxEngine {
    fn drop(&mut self) {
        // SAFETY: `self.handle` was returned by `pollux_engine_create`
        // and has not yet been destroyed — this is the only place that
        // destroys it, and it runs at most once per `PolluxEngine`.
        unsafe { ffi::pollux_engine_destroy(self.handle.as_ptr()) };
    }
}
