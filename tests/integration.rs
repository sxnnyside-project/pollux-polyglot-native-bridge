//! Integration tests for the public API, run against the real linked
//! Pollux Core artifact — never a mock. See `ARTIFACT_LAYOUT.md` for how
//! that artifact is resolved for local/CI runs.
//!
//! These prove the *wrapper*, not the ABI itself (see `pollux-ffi`'s own
//! test suite in Pollux Core for the raw-pointer-level proof). Every
//! assertion here reads a field out of a JSON string this crate received
//! verbatim from Core — this suite does not, and structurally cannot,
//! recompute a Decision itself.

use pollux_polyglot_native_bridge::{
    abi_version, check_abi_compatibility, core_version, BridgeError, PolluxEngine, PolluxStatus,
};

const MANIFEST_YAML: &str = "\
version: 1
filesystem:
  read:
    - ./assets
";

fn allow_operation_json() -> &'static str {
    r#"{"capability":"read","resource_domain":"filesystem","resource_value":"./assets"}"#
}

fn deny_operation_json() -> &'static str {
    r#"{"capability":"write","resource_domain":"filesystem","resource_value":"./assets"}"#
}

// --- Loading -----------------------------------------------------------

#[test]
fn abi_and_core_versions_are_reachable_through_the_wrapper() {
    assert_eq!(abi_version(), "pollux-abi/1");
    assert!(!core_version().is_empty());
}

#[test]
fn abi_compatibility_check_passes_against_the_linked_artifact() {
    assert!(check_abi_compatibility().is_ok());
}

#[test]
fn engine_loads_from_a_valid_manifest() {
    assert!(PolluxEngine::load(MANIFEST_YAML).is_ok());
}

#[test]
fn load_rejects_malformed_manifest() {
    let result = PolluxEngine::load("not: [valid, manifest");
    assert!(matches!(
        result,
        Err(BridgeError::Core(PolluxStatus::ManifestError))
    ));
}

// --- Evaluation ----------------------------------------------------------

#[test]
fn allowed_operation_round_trips_through_the_wrapper() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let trace_json = engine
        .evaluate(allow_operation_json())
        .expect("evaluate succeeds");

    assert!(
        trace_json.contains("\"outcome\":\"allow\""),
        "expected an allow outcome in Core's trace, got: {trace_json}"
    );
}

#[test]
fn denied_operation_round_trips_through_the_wrapper() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let trace_json = engine
        .evaluate(deny_operation_json())
        .expect("evaluate succeeds");

    assert!(
        trace_json.contains("\"outcome\":\"deny\""),
        "expected a deny outcome in Core's trace, got: {trace_json}"
    );
}

/// A Contract-inadmissible Authority (declares both `write:filesystem`
/// and `execute:process`, forbidden together by Core's own
/// `INV-AUTH-001`) still produces a normal, successful ABI call — the
/// inadmissibility surfaces as a `deny` outcome with violation detail
/// inside the trace, not as a `BridgeError`. This crate never evaluates
/// the Contract itself; it only reports what Core's trace already says.
#[test]
fn contract_denied_operation_is_a_successful_call_with_a_deny_outcome() {
    let manifest = "\
version: 1
filesystem:
  write:
    - ./app
process:
  execute:
    - build
";
    let engine = PolluxEngine::load(manifest).expect("manifest loads");
    let operation =
        r#"{"capability":"write","resource_domain":"filesystem","resource_value":"./app"}"#;

    let trace_json = engine.evaluate(operation).expect("evaluate succeeds");

    assert!(
        trace_json.contains("\"outcome\":\"deny\""),
        "expected a deny outcome, got: {trace_json}"
    );
    assert!(
        trace_json.contains("INV-AUTH-001"),
        "expected the Contract violation to be named in the trace, got: {trace_json}"
    );
}

#[test]
fn evaluate_rejects_malformed_operation_json() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let result = engine.evaluate("{not valid json");
    assert!(matches!(
        result,
        Err(BridgeError::Core(PolluxStatus::OperationError))
    ));
}

// --- Lifecycle -----------------------------------------------------------

#[test]
fn one_engine_survives_both_outcomes_then_drops_cleanly() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");

    let allow = engine.evaluate(allow_operation_json()).unwrap();
    let deny = engine.evaluate(deny_operation_json()).unwrap();

    assert!(allow.contains("\"outcome\":\"allow\""));
    assert!(deny.contains("\"outcome\":\"deny\""));
    // `engine` drops here, destroying the Core handle exactly once.
}

#[test]
fn many_engines_can_be_created_and_dropped_independently() {
    for _ in 0..8 {
        let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
        engine.evaluate(allow_operation_json()).unwrap();
    }
}

// --- Determinism -----------------------------------------------------------

#[test]
fn repeated_evaluation_of_the_same_operation_produces_identical_results() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let first = engine.evaluate(allow_operation_json()).unwrap();
    let second = engine.evaluate(allow_operation_json()).unwrap();
    let third = engine.evaluate(allow_operation_json()).unwrap();
    assert_eq!(first, second);
    assert_eq!(second, third);
}

#[test]
fn repeated_evaluation_of_a_deny_produces_identical_traces() {
    let engine = PolluxEngine::load(MANIFEST_YAML).expect("manifest loads");
    let first = engine.evaluate(deny_operation_json()).unwrap();
    let second = engine.evaluate(deny_operation_json()).unwrap();
    assert_eq!(first, second);
}
