//! NeuroLiftFoundation — the ASFDK orchestration layer (Rust port).
//!
//! Mirrors `foundation.go` from the Go port: mode-default component sets with
//! explicit override semantics (an override always wins), strict TOI
//! validation at construction, fail-closed channel provenance, canonical
//! interaction routing (crisis -> RRT, emotional assessment -> Sleepwalker
//! with canonical RRT handoff, preference update -> TOI validation), and a
//! TOI-aware health check.

use crate::dto::{
    ComponentStatus, FoundationConfig, FoundationResponse, HealthCheckResult, ToiValidationResult,
    UserInteraction,
};
use crate::promptdefense::{default_toi, validate_toi};
use crate::rrt::{assess_crisis_with_provenance, iso_timestamp_now, text_from_data};
use crate::sleepwalker::{assess_emotional_state_with_provenance, requires_rrta_handoff};
use crate::types::{Channel, FoundationMode, InteractionType};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::path::Path;

/// The governance component identifiers, in canonical order.
pub const COMPONENT_TOI_OTOI: &str = "toi_otoi_framework";
pub const COMPONENT_SLEEPWALKER: &str = "sleepwalker_protocol";
pub const COMPONENT_RRT: &str = "rrt_advocate";

/// Default active-component set for a mode (override semantics apply on top).
pub fn default_components(mode: FoundationMode) -> Map<String, Value> {
    let (toi, sleepwalker, rrt) = match mode {
        FoundationMode::Unified => (true, true, true),
        FoundationMode::CrisisOnly => (false, false, true),
        FoundationMode::ContinuityOnly => (false, true, false),
        FoundationMode::FrameworkOnly => (true, false, false),
        FoundationMode::Development => (true, true, false),
    };
    let mut m = Map::new();
    m.insert(COMPONENT_TOI_OTOI.to_string(), json!(toi));
    m.insert(COMPONENT_SLEEPWALKER.to_string(), json!(sleepwalker));
    m.insert(COMPONENT_RRT.to_string(), json!(rrt));
    m
}

/// The orchestrated foundation instance.
pub struct NeuroLiftFoundation {
    user_id: String,
    mode: FoundationMode,
    components: Map<String, Value>,
    pub(crate) toi: Value,
}

impl NeuroLiftFoundation {
    /// Builds a foundation. Fails when a configured TOI file is missing or
    /// unreadable, malformed, or semantically invalid — a foundation must
    /// never run with a degraded governance document.
    pub fn new(config: FoundationConfig) -> Result<Self, String> {
        let mode = config.mode.unwrap_or(FoundationMode::Unified);
        let mut components = default_components(mode);
        if let Some(overrides) = &config.components {
            for (key, value) in [
                (COMPONENT_TOI_OTOI, overrides.toi_otoi_framework),
                (COMPONENT_SLEEPWALKER, overrides.sleepwalker_protocol),
                (COMPONENT_RRT, overrides.rrt_advocate),
            ] {
                if let Some(v) = value {
                    components.insert(key.to_string(), json!(v));
                }
            }
        }
        let toi = match &config.toi {
            None => default_toi(),
            Some(crate::dto::ToiSource::Document(doc)) => doc.clone(),
            Some(crate::dto::ToiSource::Path(path)) => {
                if !Path::new(path).is_file() {
                    return Err(format!("TOI file not found: {path}"));
                }
                let raw = std::fs::read_to_string(path)
                    .map_err(|e| format!("TOI file unreadable: {path}: {e}"))?;
                serde_json::from_str(&raw)
                    .map_err(|e| format!("TOI file malformed: {path}: {e}"))?
            }
        };
        let result = validate_toi(Some(&toi));
        if !result.valid {
            let first = result
                .errors
                .first()
                .map(|e| e.message.clone())
                .unwrap_or_else(|| "invalid TOI document".to_string());
            return Err(format!("invalid TOI document: {first}"));
        }
        let user_id = if config.user_id.is_empty() {
            "anonymous".to_string()
        } else {
            config.user_id.clone()
        };
        Ok(NeuroLiftFoundation {
            user_id,
            mode,
            components,
            toi: result.toi.unwrap_or(toi),
        })
    }

    /// Reports whether a component is active for this foundation.
    pub fn is_component_active(&self, name: &str) -> bool {
        self.components
            .get(name)
            .and_then(Value::as_bool)
            .unwrap_or(false)
    }

    /// Validates the foundation's resolved TOI document.
    pub fn validate_toi_document(&self) -> ToiValidationResult {
        validate_toi(Some(&self.toi))
    }

    /// Per-component health. A component is healthy when it is active; the
    /// TOI/OTOI component additionally requires the foundation's resolved TOI
    /// document to pass validation (a corrupted post-construction TOI fails
    /// the health check).
    pub fn health_check(&self) -> HealthCheckResult {
        let mut healthy = true;
        let toi_valid = self.validate_toi_document().valid;
        let mut components = BTreeMap::new();
        for name in [COMPONENT_TOI_OTOI, COMPONENT_SLEEPWALKER, COMPONENT_RRT] {
            let active = self.is_component_active(name);
            let mut error = None;
            if active && name == COMPONENT_TOI_OTOI && !toi_valid {
                error = Some("TOI document invalid".to_string());
                healthy = false;
            }
            components.insert(
                name.to_string(),
                ComponentStatus {
                    active,
                    mode: self.mode.as_str().to_string(),
                    error,
                },
            );
        }
        HealthCheckResult {
            healthy,
            components,
            timestamp: iso_timestamp_now(),
        }
    }
    /// Routes an interaction through the components its interaction type and
    /// the active component set select, mirroring the canonical foundation
    /// routing:
    ///   - emotional_assessment -> Sleepwalker analysis, with an automatic RRT
    ///     handoff when the assessed state warrants it
    ///     ([requires_rrta_handoff])
    ///   - preference_update    -> TOI validation of the interaction payload;
    ///     an invalid TOI fails the interaction (mirrors canonical
    ///     update_preferences raising on invalid preferences)
    ///   - crisis_alert, emergency_escalation -> RRT assessment
    ///
    /// All analysis routes sanitize input first (flag, don't block).
    /// Interactions with unknown or missing channel provenance are rejected:
    /// they are never analyzed as if they were trusted user input.
    pub fn process_interaction(
        &self,
        interaction: UserInteraction,
    ) -> Result<FoundationResponse, String> {
        let mut response = FoundationResponse {
            timestamp: iso_timestamp_now(),
            response_type: interaction
                .interaction_type
                .map(|t| t.as_str().to_string())
                .unwrap_or_default(),
            content: json!({}),
            components_involved: Vec::new(),
            trusted: true,
            success: true,
        };
        let channel = interaction.channel.unwrap_or(Channel::Unknown);
        if channel == Channel::Unknown {
            // Security: never upgrade unknown or missing provenance to
            // user_input — that would silently grant user-input trust to
            // inputs whose origin cannot be verified. Fail closed instead.
            response.success = false;
            response.trusted = false;
            response.content["error"] = json!("unknown channel provenance");
            return Err("unknown channel provenance".to_string());
        }
        let text = text_from_data(&interaction.data);

        if self.is_component_active(COMPONENT_TOI_OTOI)
            && interaction.interaction_type == Some(InteractionType::PreferenceUpdate)
        {
            response
                .components_involved
                .push(COMPONENT_TOI_OTOI.to_string());
            let payload = interaction.data.get("toi");
            let result = validate_toi(payload);
            let errors = result.errors.len();
            response.content["toi_otoi"] = json!({ "valid": result.valid, "errors": errors });
            if !result.valid {
                response.success = false;
                response.content["error"] = json!("TOI validation failed");
            }
            // Trust policy: only user_input provenance is trusted. A
            // tool/model-supplied TOI payload must not be reported as
            // user-originated data even when it validates (Codex P2).
            response.trusted = response.trusted && channel == Channel::UserInput;
        }
        if self.is_component_active(COMPONENT_SLEEPWALKER)
            && interaction.interaction_type == Some(InteractionType::EmotionalAssessment)
        {
            response
                .components_involved
                .push(COMPONENT_SLEEPWALKER.to_string());
            let state = assess_emotional_state_with_provenance(&text, channel);
            response.trusted = response.trusted && state.trusted;
            response.content["sleepwalker"] = json!({
                "state": state.state.state,
                "confidence": state.state.confidence,
                "channel": state.channel.as_str(),
                "trusted": state.trusted,
                "flagged": state.flagged,
            });
            if self.is_component_active(COMPONENT_RRT) && requires_rrta_handoff(&state.state) {
                // Canonical handoff: Sleepwalker escalates to RRT when the
                // emotional state warrants it; rrt_advocate is listed whenever
                // the handoff was attempted.
                let assessment = assess_crisis_with_provenance(&interaction.data, channel);
                response.components_involved.push(COMPONENT_RRT.to_string());
                response.trusted = response.trusted && assessment.trusted;
                response.content["rrt"] = json!({
                    "crisis_level": assessment.assessment.crisis_level.as_str(),
                    "confidence": assessment.assessment.confidence_score,
                    "channel": assessment.channel.as_str(),
                    "trusted": assessment.trusted,
                });
            }
        }
        if self.is_component_active(COMPONENT_RRT)
            && matches!(
                interaction.interaction_type,
                Some(InteractionType::CrisisAlert) | Some(InteractionType::EmergencyEscalation)
            )
        {
            response.components_involved.push(COMPONENT_RRT.to_string());
            let assessment = assess_crisis_with_provenance(&interaction.data, channel);
            response.trusted = response.trusted && assessment.trusted;
            response.content["rrt"] = json!({
                "crisis_level": assessment.assessment.crisis_level.as_str(),
                "confidence": assessment.assessment.confidence_score,
                "channel": assessment.channel.as_str(),
                "trusted": assessment.trusted,
            });
        }

        if response.components_involved.is_empty() {
            response.success = false;
            response.content["error"] = json!(format!(
                "no components routed for interaction type {} under mode {}",
                response.response_type,
                self.mode.as_str()
            ));
        }
        Ok(response)
    }

    /// A human-readable one-line status summary.
    pub fn status_summary(&self) -> String {
        let names: Vec<&str> = [COMPONENT_TOI_OTOI, COMPONENT_SLEEPWALKER, COMPONENT_RRT]
            .into_iter()
            .filter(|name| self.is_component_active(name))
            .collect();
        format!(
            "user={} mode={} active=[{}]",
            self.user_id,
            self.mode.as_str(),
            names.join(",")
        )
    }
}
