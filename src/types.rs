//! Core enums and canonical JSON names for the ASFDK Rust port.
//!
//! Behavior mirrors the canonical TypeScript/Python reference
//! ([NeuroLift-Technologies/asfdk]) and the Go port
//! ([NeuroLift-Technologies/asfdk-go]), including canonical enum JSON names.

use serde::{Deserialize, Serialize};

/// Foundation mode: selects the default active component set of a foundation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FoundationMode {
    Unified,
    #[serde(rename = "crisis_only")]
    CrisisOnly,
    /// Serialized as `continuity` (canonical name differs from the variant).
    #[serde(rename = "continuity")]
    ContinuityOnly,
    #[serde(rename = "framework")]
    FrameworkOnly,
    Development,
}

impl FoundationMode {
    /// Parses a canonical mode name; unknown or empty input resolves to
    /// [`FoundationMode::Unified`].
    pub fn parse(s: &str) -> Self {
        match s {
            "unified" => FoundationMode::Unified,
            "crisis_only" => FoundationMode::CrisisOnly,
            "continuity" => FoundationMode::ContinuityOnly,
            "framework" => FoundationMode::FrameworkOnly,
            "development" => FoundationMode::Development,
            _ => FoundationMode::Unified,
        }
    }

    /// Canonical JSON name.
    pub fn as_str(&self) -> &'static str {
        match self {
            FoundationMode::Unified => "unified",
            FoundationMode::CrisisOnly => "crisis_only",
            FoundationMode::ContinuityOnly => "continuity",
            FoundationMode::FrameworkOnly => "framework",
            FoundationMode::Development => "development",
        }
    }
}

/// Interaction type: classifies a governed interaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionType {
    EmotionalAssessment,
    CrisisAlert,
    PreferenceUpdate,
    OptimizationRequest,
    StatusInquiry,
    EmergencyEscalation,
}

impl InteractionType {
    /// Canonical JSON name (snake_case), used verbatim as the response type.
    pub fn as_str(&self) -> &'static str {
        match self {
            InteractionType::EmotionalAssessment => "emotional_assessment",
            InteractionType::CrisisAlert => "crisis_alert",
            InteractionType::PreferenceUpdate => "preference_update",
            InteractionType::OptimizationRequest => "optimization_request",
            InteractionType::StatusInquiry => "status_inquiry",
            InteractionType::EmergencyEscalation => "emergency_escalation",
        }
    }
}

/// Provenance channel of an input or output. Only
/// [`Channel::UserInput`](Channel::UserInput) is trusted (canonical trust
/// model); [`Channel::Unknown`] is the zero-ish value and fails closed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    /// Unknown or missing provenance; fails closed.
    #[serde(rename = "")]
    Unknown,
    #[serde(rename = "user_input")]
    UserInput,
    #[serde(rename = "model_output")]
    ModelOutput,
    #[serde(rename = "tool_result")]
    ToolResult,
    #[serde(rename = "system")]
    System,
}

impl Channel {
    /// Parses a canonical channel name; unknown or empty input resolves to
    /// [`Channel::Unknown`]. Non-canonical values never pass through.
    pub fn parse(s: &str) -> Self {
        match s {
            "user_input" => Channel::UserInput,
            "model_output" => Channel::ModelOutput,
            "tool_result" => Channel::ToolResult,
            "system" => Channel::System,
            _ => Channel::Unknown,
        }
    }

    /// Canonical JSON name; [`Channel::Unknown`] serializes as `""`.
    pub fn as_str(&self) -> &'static str {
        match self {
            Channel::Unknown => "",
            Channel::UserInput => "user_input",
            Channel::ModelOutput => "model_output",
            Channel::ToolResult => "tool_result",
            Channel::System => "system",
        }
    }
}

/// Risk level for sanitization findings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

/// Security event type for the audit log.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SecurityEventType {
    InjectionAttempt,
    ValidationFailure,
    LengthExceeded,
}

/// Output schema selector for output validation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputSchemaType {
    Json,
    Text,
}

/// Ordered crisis severity; comparisons such as `level >= CrisisLevel::Red`
/// are meaningful. [`CrisisLevel::Green`] is the least severe.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CrisisLevel {
    Green,
    Yellow,
    Orange,
    Red,
    Black,
}

impl CrisisLevel {
    /// Canonical JSON name.
    pub fn as_str(&self) -> &'static str {
        match self {
            CrisisLevel::Green => "green",
            CrisisLevel::Yellow => "yellow",
            CrisisLevel::Orange => "orange",
            CrisisLevel::Red => "red",
            CrisisLevel::Black => "black",
        }
    }

    /// Parses a canonical level name; unknown input resolves to
    /// [`CrisisLevel::Green`].
    pub fn parse(s: &str) -> Self {
        match s.to_ascii_lowercase().as_str() {
            "yellow" => CrisisLevel::Yellow,
            "orange" => CrisisLevel::Orange,
            "red" => CrisisLevel::Red,
            "black" => CrisisLevel::Black,
            _ => CrisisLevel::Green,
        }
    }
}
