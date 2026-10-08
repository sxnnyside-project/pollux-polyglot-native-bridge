//! SDK-foundation validation.
//!
//! Where `tests/integration.rs` proves the raw `evaluate(&str) -> String`
//! ABI wrapper against the real Core artifact, this file proves the
//! *SDK-facing* surface on top of it: an external Rust application using
//! only [`Operation`]/[`Evaluation`]/[`Outcome`] — never a hand-built
//! `OperationWire` JSON string, never `pollux_polyglot_native_bridge::PolluxStatus`'s
//! raw ABI details, never anything from the crate's private `ffi` module.
//!
//! ```text
//! Rust application -> Bridge SDK API -> real Pollux Core v0.1.0 -> ALLOW/DENY + trace
//! ```
//!
//! Still runs against the real, remotely-downloaded `pollux-core v0.1.0`
//! artifact `build.rs` resolves — no mock, no parallel policy model.

use pollux_polyglot_native_bridge::{BridgeError, Operation, Outcome, PolluxEngine, PolluxStatus};

const MANIFEST_YAML: &str = "\
version: 1
filesystem:
  read:
    - ./assets
";

#[test]
fn an_allowed_operation_is_expressed_and_read_without_touching_raw_wire_json() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");

    let operation = Operation::new("read", "filesystem", "./assets/report.txt");
    let evaluation = engine
        .evaluate_operation(operation)
        .expect("evaluate_operation succeeds");

    assert_eq!(evaluation.outcome(), Some(Outcome::Allow));
    // The full Core trace is still reachable — the SDK view adds an
    // accessor, it does not hide or replace what Core actually returned.
    assert!(evaluation.trace_json().contains("\"outcome\":\"allow\""));
}

#[test]
fn a_denied_operation_is_expressed_and_read_without_touching_raw_wire_json() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");

    let operation = Operation::new("write", "filesystem", "./assets/report.txt");
    let evaluation = engine
        .evaluate_operation(operation)
        .expect("evaluate_operation succeeds");

    assert_eq!(evaluation.outcome(), Some(Outcome::Deny));
}

/// The exact scenario described in the task: an application that only
/// ever imports the SDK-facing names, builds a real Manifest-governed
/// `Operation`, and branches on `Outcome` — structurally incapable of
/// hand-rolling a Decision itself, because it never sees a `Decision`,
/// `Capability`, `Resource`, or `Contract` type at all.
#[test]
fn a_rust_application_can_consume_the_bridge_as_a_native_sdk_boundary() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");

    let requests = [
        Operation::new("read", "filesystem", "./assets/allowed.txt"),
        Operation::new("read", "filesystem", "./secrets/denied.txt"),
    ];

    let mut outcomes = Vec::new();
    for request in requests {
        let evaluation = engine
            .evaluate_operation(request)
            .expect("evaluate_operation succeeds");
        outcomes.push(evaluation.outcome().expect("Core's trace has an outcome"));
    }

    assert_eq!(outcomes, [Outcome::Allow, Outcome::Deny]);
}

#[test]
fn operation_construction_never_normalizes_or_validates_its_own_strings() {
    // `Operation` is a plain data carrier over Core's own wire fields —
    // it performs no validation of its own. An unrecognized capability
    // string is Core's rejection to make, and Core makes it as an ABI-level
    // `OperationError` (a `BridgeError`, not a `deny` outcome) — this SDK
    // layer neither pre-empts that check nor reinterprets Core's answer
    // into something else.
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let operation = Operation::new("not-a-real-capability", "filesystem", "./assets");
    let result = engine.evaluate_operation(operation);
    assert!(matches!(
        result,
        Err(BridgeError::Core(PolluxStatus::OperationError))
    ));
}
