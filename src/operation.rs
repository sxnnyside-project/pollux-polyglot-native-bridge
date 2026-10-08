//! A typed, ergonomic view over Core's own `OperationWire` /
//! `EvaluationTraceWire` wire shapes (`pollux-protocol/1`) — the exact
//! shapes already exercised in `tests/integration.rs` as raw JSON
//! strings, given field names here for a native SDK consumer instead of
//! requiring every caller to hand-assemble JSON.
//!
//! This module does not define a parallel policy model. [`Operation`]'s
//! three fields (`capability`, `resource_domain`, `resource_value`) are
//! exactly Core's own `OperationWire` fields, using exactly the strings
//! Core's own domain types already produce (`"read"`, `"filesystem"`,
//! ...) — this crate does not invent, validate, or normalize them; Core
//! does that when it receives the JSON. Likewise [`Outcome`] is read
//! directly out of the `outcome` field Core already put in its trace; it
//! is not computed here and this crate still has no `Decision` type of
//! its own.

use std::fmt;

/// One `(Capability, Resource)` pair to ask Core about — the same shape
/// `pollux-cli` and every other Authority Protocol client already use.
/// Construct with the exact strings Core's own domain types produce
/// (`"read"`, `"write"`, ... for capability; `"filesystem"`,
/// `"network"`, ... for resource domain) — this type does not validate
/// or interpret them, Core does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation<'a> {
    pub capability: &'a str,
    pub resource_domain: &'a str,
    pub resource_value: &'a str,
}

impl<'a> Operation<'a> {
    pub fn new(capability: &'a str, resource_domain: &'a str, resource_value: &'a str) -> Self {
        Self {
            capability,
            resource_domain,
            resource_value,
        }
    }

    /// The exact `OperationWire` JSON shape Core's own wire protocol
    /// defines — field-for-field, string-for-string identical to what
    /// `tests/integration.rs` already sends by hand.
    pub(crate) fn to_wire_json(self) -> String {
        format!(
            r#"{{"capability":"{}","resource_domain":"{}","resource_value":"{}"}}"#,
            json_escape(self.capability),
            json_escape(self.resource_domain),
            json_escape(self.resource_value)
        )
    }
}

fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Core's own answer to one [`Operation`] — the exact `outcome` string
/// already present in the `EvaluationTraceWire` JSON
/// [`crate::PolluxEngine::evaluate`] returns. Not computed by this
/// crate; only read out of Core's own trace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Outcome {
    Allow,
    Deny,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Allow => "allow",
            Self::Deny => "deny",
        })
    }
}

/// The result of [`crate::PolluxEngine::evaluate_operation`]: Core's
/// `EvaluationTraceWire` JSON, unmodified, plus a convenience accessor
/// for the `outcome` field already inside it.
///
/// [`Evaluation::trace_json`] is always the full, verbatim trace — this
/// type never discards information Core returned, it only adds a
/// read-only view over one field already present in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    trace_json: String,
}

impl Evaluation {
    pub(crate) fn from_trace_json(trace_json: String) -> Self {
        Self { trace_json }
    }

    /// The full `EvaluationTraceWire` JSON Core produced, byte-for-byte.
    pub fn trace_json(&self) -> &str {
        &self.trace_json
    }

    /// Reads the `outcome` field out of [`Self::trace_json`] by
    /// substring search — the same technique `tests/integration.rs`
    /// already uses directly. `None` means the trace's shape was not
    /// recognized at all (an infrastructure concern, not a third
    /// outcome); callers that need fail-closed behavior must treat
    /// `None` the same as [`Outcome::Deny`], never as an implicit allow.
    pub fn outcome(&self) -> Option<Outcome> {
        if self.trace_json.contains("\"outcome\":\"allow\"") {
            Some(Outcome::Allow)
        } else if self.trace_json.contains("\"outcome\":\"deny\"") {
            Some(Outcome::Deny)
        } else {
            None
        }
    }
}
