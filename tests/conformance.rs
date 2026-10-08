//! Exercises the Pollux Authority Protocol v1 conformance suite through
//! the Pollux Polyglot Native Bridge wrapper.

use std::fs;
use std::path::PathBuf;

use pollux_polyglot_native_bridge::{Operation, Outcome, PolluxEngine};
use serde::Deserialize;

fn find_conformance_dir() -> Option<PathBuf> {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let candidates = [
        manifest_dir.join("../../Pollux/conformance"),
        manifest_dir.join("../Pollux/conformance"),
    ];

    candidates.into_iter().find(|candidate| candidate.is_dir())
}

#[derive(Deserialize)]
struct EvaluationVectorWire {
    capability: String,
    resource_domain: String,
    resource_value: String,
}

#[derive(Deserialize)]
struct EvaluationVector {
    id: String,
    description: String,
    manifest_yaml: String,
    operation: EvaluationVectorWire,
    expected_outcome: String,
}

#[test]
fn run_conformance_evaluations_through_native_bridge() {
    let Some(conformance_root) = find_conformance_dir() else {
        eprintln!("warning: Conformance suite directory not found; skipping conformance runner");
        return;
    };

    let dir = conformance_root.join("evaluations");
    let entries = fs::read_dir(&dir).expect("failed to read evaluations directory");

    let mut vector_count = 0;
    for entry in entries {
        let path = entry.expect("valid entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }

        let content = fs::read_to_string(&path).expect("read vector file");
        let vector: EvaluationVector =
            serde_json::from_str(&content).expect("parse evaluation vector JSON");

        let engine = PolluxEngine::load(&vector.manifest_yaml).unwrap_or_else(|e| {
            panic!(
                "vector `{}` ({}) failed to load manifest across bridge: {e}",
                vector.id, vector.description
            )
        });

        let op = Operation::new(
            &vector.operation.capability,
            &vector.operation.resource_domain,
            &vector.operation.resource_value,
        );

        let evaluation = engine.evaluate_operation(op).unwrap_or_else(|e| {
            panic!(
                "vector `{}` failed to evaluate operation across bridge: {e}",
                vector.id
            )
        });

        let expected_outcome = match vector.expected_outcome.as_str() {
            "allow" => Outcome::Allow,
            "deny" => Outcome::Deny,
            other => panic!("unknown expected outcome: {other}"),
        };

        assert_eq!(
            evaluation.outcome(),
            Some(expected_outcome),
            "vector `{}` ({}) outcome mismatch across native bridge",
            vector.id,
            vector.description
        );

        vector_count += 1;
    }

    assert!(
        vector_count >= 8,
        "expected at least 8 evaluation vectors, ran {vector_count}"
    );
}
