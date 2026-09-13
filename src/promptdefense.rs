//! TOI/OTOI validation, prompt-injection defense, and the security audit log.
//!
//! Mirrors `promptdefense.go` from the Go port (canonical asfdk behavior):
//! strict type/value validation, sanitize-first flag-not-block assessment,
//! and newline-delimited JSON security events.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::{json, Value};

use crate::dto::{
    OtoiValidationResult, SanitizationResult, SecurityEvent, ToiValidationResult, ValidationIssue,
    ValidationResult,
};
use crate::types::{OutputSchemaType, RiskLevel, SecurityEventType};

/// Default hard cap for [`sanitize_input`].
pub const MAX_INPUT_LENGTH: usize = 10_000;

/// Known prompt-injection substrings (lowercase).
const INJECTION_PATTERNS: &[&str] = &[
    "ignore all previous instructions",
    "ignore previous instructions",
    "disregard all prior instructions",
    "forget all previous instructions",
    "system prompt:",
    "you are now",
    "act as if you have no",
    "pretend you have no",
    "override your instructions",
    "bypass your safety",
    "reveal your system prompt",
    "print your instructions",
    "repeat your system prompt",
];

/// The default TOI document, used when no TOI is configured.
pub fn default_toi() -> Value {
    json!({
        "version": "1.0",
        "framework": "ASFDK",
        "description": "Default Terms of Interaction for governed AI sessions",
        "respect_autonomy": true,
        "no_harm": true,
        "privacy_first": true,
        "transparency": true,
        "user_sovereignty": true
    })
}

/// The default OTOI charter document.
pub fn default_charter() -> Value {
    json!({
        "version": "1.0",
        "charter": "OTOI",
        "description": "Default operating charter for AI agents",
        "transparency": true,
        "accountability": true,
        "fairness": true,
        "non_maleficence": true
    })
}

/// Human-readable serde_json type name for error messages.
fn json_type_name(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

fn issue(message: String, path: &str, code: &str) -> ValidationIssue {
    ValidationIssue {
        message,
        path: path.to_string(),
        code: code.to_string(),
    }
}

/// Validates a TOI document and returns it normalized on success. `None` or
/// `Value::Null` validates against the default TOI. Validation is strict:
/// field types and values are checked, not just key presence — a document
/// like `{"version":1,"respect_autonomy":"yes","no_harm":null}` fails.
pub fn validate_toi(toi: Option<&Value>) -> ToiValidationResult {
    let resolved: Value = match toi {
        None | Some(Value::Null) => default_toi(),
        Some(doc) => doc.clone(),
    };
    let mut errs: Vec<ValidationIssue> = Vec::new();
    match resolved.get("version") {
        None | Some(Value::Null) => errs.push(issue(
            "missing required field 'version'".into(),
            "version",
            "missing_field",
        )),
        Some(Value::String(s)) if s == "1.0" => {}
        Some(Value::String(s)) => errs.push(issue(
            format!("unsupported TOI version {s:?}, want \"1.0\""),
            "version",
            "invalid_value",
        )),
        Some(v) => errs.push(issue(
            format!(
                "field 'version' must be a string, got {}",
                json_type_name(v)
            ),
            "version",
            "invalid_type",
        )),
    }
    for field in ["respect_autonomy", "no_harm"] {
        match resolved.get(field) {
            None | Some(Value::Null) => errs.push(issue(
                format!("missing required field '{field}'"),
                field,
                "missing_field",
            )),
            Some(Value::Bool(true)) => {}
            Some(Value::Bool(false)) => errs.push(issue(
                format!("field '{field}' must be true"),
                field,
                "invalid_value",
            )),
            Some(v) => errs.push(issue(
                format!(
                    "field '{field}' must be a boolean, got {}",
                    json_type_name(v)
                ),
                field,
                "invalid_type",
            )),
        }
    }
    if !errs.is_empty() {
        return ToiValidationResult {
            valid: false,
            errors: errs,
            toi: None,
        };
    }
    ToiValidationResult {
        valid: true,
        errors: Vec::new(),
        toi: Some(resolved),
    }
}

/// Validates an OTOI charter document. `None` validates against the default
/// charter. Like [`validate_toi`], validation is strict about types and
/// values.
pub fn validate_charter(charter: Option<&Value>) -> OtoiValidationResult {
    let resolved: Value = match charter {
        None | Some(Value::Null) => default_charter(),
        Some(doc) => doc.clone(),
    };
    let mut errs: Vec<ValidationIssue> = Vec::new();
    match resolved.get("version") {
        None | Some(Value::Null) => errs.push(issue(
            "missing required field 'version'".into(),
            "version",
            "missing_field",
        )),
        Some(Value::String(s)) if s == "1.0" => {}
        Some(Value::String(s)) => errs.push(issue(
            format!("unsupported charter version {s:?}, want \"1.0\""),
            "version",
            "invalid_value",
        )),
        Some(v) => errs.push(issue(
            format!(
                "field 'version' must be a string, got {}",
                json_type_name(v)
            ),
            "version",
            "invalid_type",
        )),
    }
    for field in [
        "transparency",
        "accountability",
        "fairness",
        "non_maleficence",
    ] {
        match resolved.get(field) {
            None | Some(Value::Null) => errs.push(issue(
                format!("missing required field '{field}'"),
                field,
                "missing_field",
            )),
            Some(Value::Bool(true)) => {}
            Some(Value::Bool(false)) => errs.push(issue(
                format!("field '{field}' must be true"),
                field,
                "invalid_value",
            )),
            Some(v) => errs.push(issue(
                format!(
                    "field '{field}' must be a boolean, got {}",
                    json_type_name(v)
                ),
                field,
                "invalid_type",
            )),
        }
    }
    if !errs.is_empty() {
        return OtoiValidationResult {
            valid: false,
            errors: errs,
            charter: None,
        };
    }
    OtoiValidationResult {
        valid: true,
        errors: Vec::new(),
        charter: Some(resolved),
    }
}

/// Percent-decodes a string the way Go's `url.QueryUnescape` does:
/// `%XX` hex escapes and `+` become the decoded byte / space. Returns `None`
/// when the input contains a malformed escape (truncated or non-hex), in
/// which case the caller keeps the original text.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' => {
                let hi = (bytes.get(i + 1).copied()? as char).to_digit(16)?;
                let lo = (bytes.get(i + 2).copied()? as char).to_digit(16)?;
                out.push((hi * 16 + lo) as u8);
                i += 3;
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8(out).ok()
}

/// Removes zero-width and bidirectional-override characters used to smuggle
/// instructions past naive filters (Go regex `[\x{200B}-\x{200F}...]`).
fn strip_zero_width(s: &str) -> String {
    s.chars()
        .filter(|c| {
            !matches!(*c as u32,
                0x200B..=0x200F | 0x202A..=0x202E | 0x2060..=0x2064 | 0xFEFF)
        })
        .collect()
}

/// Removes ASCII control characters except tab, newline, and carriage return
/// (Go regex `[\x00-\x08\x0B\x0C\x0E-\x1F\x7F]`).
fn strip_controls(s: &str) -> String {
    s.chars()
        .filter(|c| {
            let u = *c as u32;
            !(u <= 0x08 || u == 0x0B || u == 0x0C || (0x0E..=0x1F).contains(&u) || u == 0x7F)
        })
        .collect()
}

/// Counts structural tokens (`< > [ ] { } | $`) whose presence in bulk is a
/// prompt-structure probe.
fn structural_token_count(s: &str) -> usize {
    s.chars()
        .filter(|c| matches!(c, '<' | '>' | '[' | ']' | '{' | '}' | '|' | '$'))
        .count()
}

/// Validates and sanitizes text for agent consumption. `max_length == 0`
/// falls back to [`MAX_INPUT_LENGTH`].
pub fn sanitize_input(input: &str, max_length: usize) -> SanitizationResult {
    let max_length = if max_length == 0 {
        MAX_INPUT_LENGTH
    } else {
        max_length
    };
    let fail = |reason: String, level: RiskLevel| SanitizationResult {
        clean: false,
        content: String::new(),
        reason: Some(reason),
        risk_level: level,
    };
    if input.len() > max_length {
        return fail(
            format!("input exceeds maximum length of {max_length}"),
            RiskLevel::Medium,
        );
    }
    if input.is_empty() {
        return fail("empty input".into(), RiskLevel::Low);
    }

    let mut clean = strip_zero_width(input);
    clean = strip_controls(&clean);

    let mut risk = RiskLevel::Low;
    let mut reasons: Vec<String> = Vec::new();
    if clean.contains('%') {
        if let Some(decoded) = percent_decode(&clean) {
            if decoded != clean {
                clean = decoded;
                risk = RiskLevel::Medium;
                reasons.push("encoded content detected and decoded".to_string());
            }
        }
    }
    if structural_token_count(&clean) >= 3 {
        risk = RiskLevel::High;
        reasons.push("multiple structural tokens detected".to_string());
    }
    let lower = clean.to_lowercase();
    for pattern in INJECTION_PATTERNS {
        if lower.contains(pattern) {
            risk = RiskLevel::High;
            reasons.push(format!("injection pattern detected: {pattern}"));
            break;
        }
    }
    if reasons.is_empty() {
        return SanitizationResult {
            clean: true,
            content: clean,
            reason: None,
            risk_level: risk,
        };
    }
    SanitizationResult {
        clean: false,
        content: clean,
        reason: Some(reasons.join("; ")),
        risk_level: risk,
    }
}

/// Validates an agent output string against the schema type.
/// [`OutputSchemaType::Json`] requires parseable JSON;
/// [`OutputSchemaType::Text`] requires non-empty content.
pub fn validate_output(output: &str, schema_type: OutputSchemaType) -> ValidationResult {
    match schema_type {
        OutputSchemaType::Json => match serde_json::from_str::<Value>(output) {
            Ok(_) => ValidationResult {
                valid: true,
                reason: None,
            },
            Err(_) => ValidationResult {
                valid: false,
                reason: Some("output is not valid JSON".to_string()),
            },
        },
        OutputSchemaType::Text => {
            if output.trim().is_empty() {
                ValidationResult {
                    valid: false,
                    reason: Some("empty output".to_string()),
                }
            } else {
                ValidationResult {
                    valid: true,
                    reason: None,
                }
            }
        }
    }
}

/// Path of the security audit log used by
/// [`sanitize_for_assessment`]. Overridable (tests redirect it to keep
/// writes out of the repository).
static SECURITY_LOG_PATH: std::sync::Mutex<String> = std::sync::Mutex::new(String::new());

/// Overrides the security audit log path (empty string restores the default
/// `asfdk-security.jsonl`).
pub fn set_security_log_path(path: &str) {
    *SECURITY_LOG_PATH.lock().unwrap_or_else(|e| e.into_inner()) = path.to_string();
}

fn security_log_path() -> String {
    let guard = SECURITY_LOG_PATH.lock().unwrap_or_else(|e| e.into_inner());
    if guard.is_empty() {
        "asfdk-security.jsonl".to_string()
    } else {
        guard.clone()
    }
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Appends an event as newline-delimited JSON to `path`, creating parent
/// directories (platform-aware via [`Path`]).
pub fn store_security_event(path: &str, event: &SecurityEvent) -> std::io::Result<()> {
    if let Some(dir) = Path::new(path).parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }
    let mut f = OpenOptions::new().create(true).append(true).open(path)?;
    let line = serde_json::to_vec(event).map_err(std::io::Error::other)?;
    f.write_all(&line)?;
    f.write_all(b"\n")
}

/// Builds a timestamped audit event.
pub fn new_security_event(
    event_type: SecurityEventType,
    user_id: &str,
    details: &str,
) -> SecurityEvent {
    SecurityEvent {
        event_type,
        user_id: user_id.to_string(),
        details: details.to_string(),
        timestamp: now_unix(),
    }
}

/// Sanitizes text before assessment, mirroring the canonical fail-open
/// policy: a flagged input is still assessed defensively (so a genuine
/// crisis signal is never silently suppressed by an injection heuristic) but
/// the flag and a security event are recorded.
pub fn sanitize_for_assessment(text: &str) -> SanitizationResult {
    let res = sanitize_input(text, MAX_INPUT_LENGTH);
    if res.clean {
        return res;
    }
    let event_type = match res.risk_level {
        RiskLevel::High => SecurityEventType::InjectionAttempt,
        RiskLevel::Medium => SecurityEventType::LengthExceeded,
        _ => SecurityEventType::ValidationFailure,
    };
    let _ = store_security_event(
        &security_log_path(),
        &new_security_event(event_type, "unknown", res.reason.as_deref().unwrap_or("")),
    );
    res
}
