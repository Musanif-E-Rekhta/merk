//! Cardinality-safe helpers used by the GraphQL metrics + logging extensions.
//!
//! - [`resolve_operation_label`] maps a client-supplied `operationName` to a
//!   bounded label set: a known op name, `"unknown"`, or `"anonymous"`.
//! - [`classify_error`] buckets `ServerError`s into a small enum of classes
//!   suitable as a metric attribute.
//!
//! Without these the metrics backend's cardinality is unbounded — a buggy
//! or hostile client could mint a unique series per request.

use async_graphql::ServerError;

include!(concat!(env!("OUT_DIR"), "/known_ops.rs"));

/// Bounded label for the `operation` attribute on GraphQL metrics.
///
/// Returns a `&'static str` pulled straight from the phf set so the label
/// can live in `KeyValue` without an allocation.
pub fn resolve_operation_label(name: Option<&str>) -> &'static str {
    match name {
        None => "anonymous",
        Some(n) => KNOWN_OPS
            .get_key(n)
            .copied()
            .unwrap_or("unknown"),
    }
}

/// Returns true when an operation name was supplied but isn't in the
/// allowlist — used to bump the `operations_rejected_total` counter.
pub fn is_unknown_operation(name: Option<&str>) -> bool {
    matches!(name, Some(n) if !KNOWN_OPS.contains(n))
}

/// Bucket for the `error_class` attribute on `graphql_operation_errors_total`.
///
/// Classification is conservative: validation errors are spotted by the
/// `path` being empty (parse/validate runs before any resolver). Beyond
/// that we read the `code` extension that resolvers stamp on errors via
/// `Error::new(...).extend_with(...)`. Anything unrecognised is
/// `internal` so unhandled bugs stay visible.
pub fn classify_error(err: &ServerError) -> &'static str {
    if err.path.is_empty() {
        return "validation";
    }

    if let Some(code) = err
        .extensions
        .as_ref()
        .and_then(|ext| ext.get("code"))
        .and_then(|v| match v {
            async_graphql::Value::String(s) => Some(s.as_str()),
            _ => None,
        })
    {
        return match code {
            "UNAUTHORIZED" | "FORBIDDEN" => "unauthorized",
            "NOT_FOUND" => "not_found",
            "DOWNSTREAM" => "downstream",
            _ => "internal",
        };
    }

    // Fall back to message-prefix heuristics for resolvers that haven't been
    // converted to use error extensions yet. Keep this list tiny.
    let msg = err.message.as_str();
    if msg.starts_with("Unauthorized") {
        "unauthorized"
    } else if msg.starts_with("Not found") {
        "not_found"
    } else {
        "internal"
    }
}
