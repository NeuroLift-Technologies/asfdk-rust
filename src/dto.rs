//! Data-transfer structures for the ASFDK Rust port.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::types::{
    Channel, CrisisLevel, FoundationMode, InteractionType, RiskLevel, SecurityEventType,
};

/// Per-component overrides. `None` defers to the mode default; an explicit
/// `Some(true)`/`Some(false)` always wins.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FoundationComponents {
    #[serde(
        rename = "toi_otoi_framework",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub toi_otoi_framework: Option<bool>,
    #[serde(
        rename = "sleepwalker_protocol",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub sleepwalker_protocol: Option<bool>,
    #[serde(
        rename = "rrt_advocate",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub rrt_advocate: Option<bool>,
}

/// A TOI source: either a file path to load at construction, or an inline
/// TOI document. `None` selects the default TOI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ToiSource {
    /// Path to a JSON TOI file on disk.
    Path(String),
    /// Inline TOI document.
    Document(Value),
}

/// Configuration for a foundation instance.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FoundationConfig {
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub mode: Option<FoundationMode>,
    #[serde(default)]
    pub components: Option<FoundationComponents>,
    /// `None` (default TOI), a path (`ToiSource::Path`), or an inline
    /// document (`ToiSource::Document`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub toi: Option<ToiSource>,
}

/// A single governed interaction.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UserInteraction {
    /// ISO-8601 timestamp of the interaction (caller-provided).
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(rename = "interaction_type", default)]
    pub interaction_type: Option<InteractionType>,
    #[serde(default)]
    pub data: Value,
    #[serde(default)]
    pub user_id: String,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub priority: Option<i64>,
    #[serde(default)]
    pub context: Option<Value>,
    #[serde(default)]
    pub channel: Option<Channel>,
}

/// The unified output of
/// [`NeuroLiftFoundation::process_interaction`](crate::foundation::NeuroLiftFoundation::process_interaction).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FoundationResponse {
    /// RFC-3339 UTC timestamp of processing.
    pub timestamp: String,
    /// Mirrors the interaction type (canonical semantics).
    #[serde(rename = "response_type")]
    pub response_type: String,
    pub content: Value,
    #[serde(rename = "components_involved")]
    pub components_involved: Vec<String>,
    /// Aggregate provenance trust: false when any involved component flagged
    /// the input as untrusted.
    pub trusted: bool,
    pub success: bool,
}

/// Status of one framework component.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentStatus {
    pub active: bool,
    pub mode: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregated component statuses.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckResult {
    pub healthy: bool,
    pub components: std::collections::BTreeMap<String, ComponentStatus>,
    pub timestamp: String,
}

/// Outcome of [`sanitize_input`](crate::promptdefense::sanitize_input).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SanitizationResult {
    pub clean: bool,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    #[serde(rename = "risk_level")]
    pub risk_level: RiskLevel,
}

/// Outcome of output validation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// A security audit record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityEvent {
    #[serde(rename = "event_type")]
    pub event_type: SecurityEventType,
    #[serde(rename = "user_id")]
    pub user_id: String,
    pub details: String,
    /// Unix epoch seconds.
    pub timestamp: u64,
}

/// A single schema violation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationIssue {
    pub message: String,
    pub path: String,
    pub code: String,
}

/// Outcome of [`validate_toi`](crate::promptdefense::validate_toi).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToiValidationResult {
    pub valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<ValidationIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toi: Option<Value>,
}

/// Outcome of [`validate_charter`](crate::promptdefense::validate_charter).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OtoiValidationResult {
    pub valid: bool,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub errors: Vec<ValidationIssue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub charter: Option<Value>,
}

/// Sleepwalker emotional analysis output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalState {
    /// Category name: `distress`, `anxiety`, `sadness`, `anger`, `crisis`,
    /// `positive`, or `neutral`.
    pub state: String,
    pub confidence: f64,
    pub indicators: Vec<String>,
    #[serde(rename = "raw_scores")]
    pub raw_scores: std::collections::BTreeMap<String, f64>,
}

/// Emotional analysis with channel-trust provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionalStateWithProvenance {
    #[serde(flatten)]
    pub state: EmotionalState,
    pub channel: Channel,
    pub trusted: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub flagged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_reason: Option<String>,
}

/// RRT crisis assessment output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisAssessment {
    /// RFC-3339 UTC timestamp of the assessment.
    pub timestamp: String,
    #[serde(rename = "crisis_level")]
    pub crisis_level: CrisisLevel,
    #[serde(rename = "primary_indicators")]
    pub primary_indicators: Vec<String>,
    #[serde(rename = "secondary_indicators")]
    pub secondary_indicators: Vec<String>,
    #[serde(rename = "confidence_score")]
    pub confidence_score: f64,
    #[serde(rename = "estimated_duration", skip_serializing_if = "Option::is_none")]
    pub estimated_duration: Option<f64>,
    #[serde(rename = "recommended_interventions")]
    pub recommended_interventions: Vec<String>,
    #[serde(rename = "escalation_threshold")]
    pub escalation_threshold: f64,
    #[serde(rename = "user_safety_score")]
    pub user_safety_score: f64,
    #[serde(rename = "context_factors")]
    pub context_factors: Value,
}

/// Crisis assessment with channel-trust provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrisisAssessmentWithProvenance {
    #[serde(flatten)]
    pub assessment: CrisisAssessment,
    pub channel: Channel,
    pub trusted: bool,
    #[serde(skip_serializing_if = "std::ops::Not::not", default)]
    pub flagged: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flag_reason: Option<String>,
}
