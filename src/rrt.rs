//! RRT Advocate — crisis assessment with additive-weight scoring.
//!
//! Mirrors the reviewed Go port exactly: pattern table, per-indicator weights,
//! one weight per matched indicator name, and recalibrated thresholds
//! (green 0.1 / yellow 0.2 / orange 0.3 / red 0.4) so a single explicit
//! self-harm ideation match (0.4) reaches [`CrisisLevel::Black`].
//! Sanitize-first and flag-not-block, per the canonical assess_text semantics.

use crate::dto::{CrisisAssessment, CrisisAssessmentWithProvenance};
use crate::promptdefense::sanitize_for_assessment;
use crate::types::{Channel, CrisisLevel};
use serde_json::{json, Map, Value};

/// One crisis-speak indicator with its additive weight.
struct CrisisPattern {
    name: &'static str,
    weight: f64,
    patterns: &'static [&'static str],
}

/// Canonical crisis indicators in sorted-name order (deterministic scoring).
static CRISIS_PATTERNS: &[CrisisPattern] = &[
    CrisisPattern {
        name: "acute_agitation",
        weight: 0.08,
        patterns: &[
            "can't calm down",
            "can\u{2019}t calm down",
            "shaking with anger",
            "about to explode",
        ],
    },
    CrisisPattern {
        name: "burden_statements",
        weight: 0.15,
        patterns: &[
            "burden on everyone",
            "everyone would be better off without me",
            "people would be happier if i was gone",
        ],
    },
    CrisisPattern {
        name: "goodbye_statements",
        weight: 0.12,
        patterns: &[
            "this is goodbye",
            "final message",
            "last time talking",
            "goodbye forever",
        ],
    },
    CrisisPattern {
        name: "giving_away_possessions",
        weight: 0.08,
        patterns: &[
            "giving away my things",
            "giving away my stuff",
            "gave away everything",
        ],
    },
    CrisisPattern {
        name: "hopelessness",
        weight: 0.15,
        patterns: &[
            "no point",
            "no reason to live",
            "hopeless",
            "nothing matters",
            "never get better",
        ],
    },
    CrisisPattern {
        name: "self_harm_ideation",
        weight: 0.4,
        patterns: &[
            "kill myself",
            "end my life",
            "suicidal",
            "want to die",
            "better off dead",
            "hurt myself",
            "harm myself",
        ],
    },
    CrisisPattern {
        name: "substance_concern",
        weight: 0.1,
        patterns: &[
            "overdose",
            "took too many",
            "drinking to forget",
            "pills to sleep",
        ],
    },
];

/// Ordered pattern names (deterministic iteration for scoring/tests).
pub fn crisis_pattern_names() -> Vec<&'static str> {
    CRISIS_PATTERNS.iter().map(|p| p.name).collect()
}

/// Extract the "text" field, treating missing or null values as empty
/// (never feed "<nil>"-style placeholders to the analyzers).
pub fn text_from_data(data: &Value) -> String {
    match data.get("text") {
        Some(Value::String(s)) => s.clone(),
        _ => String::new(),
    }
}

/// Classify an additive score into a crisis level (Go-calibrated thresholds).
fn level_for_score(score: f64) -> CrisisLevel {
    if score >= 0.4 {
        CrisisLevel::Black
    } else if score >= 0.3 {
        CrisisLevel::Red
    } else if score >= 0.2 {
        CrisisLevel::Orange
    } else if score >= 0.1 {
        CrisisLevel::Yellow
    } else {
        CrisisLevel::Green
    }
}

/// Recommended interventions for a crisis level (Go scripts).
pub fn recommended_interventions(level: CrisisLevel) -> Vec<&'static str> {
    match level {
        CrisisLevel::Green => vec!["routine monitoring"],
        CrisisLevel::Yellow => vec!["supportive engagement", "offer coping strategies"],
        CrisisLevel::Orange => vec![
            "active listening",
            "provide crisis hotline resources",
            "increase monitoring frequency",
        ],
        CrisisLevel::Red | CrisisLevel::Black => vec![
            "immediate human escalation",
            "provide crisis hotline resources",
            "encourage contact with emergency services",
        ],
    }
}

/// RFC-3339 UTC timestamp with second precision (std-only implementation).
pub fn iso_timestamp_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86_400) as i64;
    let rem = secs % 86_400;
    let (h, m, s) = (rem / 3_600, (rem % 3_600) / 60, rem % 60);
    // Civil-from-days algorithm (Howard Hinnant) — valid for the full range.
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = if month <= 2 { y + 1 } else { y };
    format!("{year:04}-{month:02}-{d:02}T{h:02}:{m:02}:{s:02}Z")
}

/// Assess crisis level from interaction data (no provenance).
pub fn assess_crisis(data: &Value) -> CrisisAssessment {
    let text = text_from_data(data);
    let lowered = text.to_lowercase();
    let mut primary: Vec<String> = Vec::new();
    let mut score = 0.0_f64;
    for pattern in CRISIS_PATTERNS {
        for p in pattern.patterns {
            if lowered.contains(*p) {
                score += pattern.weight;
                primary.push(pattern.name.to_string());
                break; // one weight per indicator name, like the Go port
            }
        }
    }
    let score = score.clamp(0.0, 1.0);
    let level = level_for_score(score);
    let mut confidence = score;
    if primary.is_empty() {
        confidence = 0.0;
    } else if confidence > 0.95 {
        confidence = 0.95;
    }
    CrisisAssessment {
        timestamp: iso_timestamp_now(),
        crisis_level: level,
        primary_indicators: primary,
        secondary_indicators: Vec::new(),
        confidence_score: confidence,
        estimated_duration: None,
        recommended_interventions: recommended_interventions(level)
            .into_iter()
            .map(String::from)
            .collect(),
        escalation_threshold: 0.3,
        user_safety_score: 1.0 - confidence,
        context_factors: Value::Object(Map::new()),
    }
}

/// Assess crisis level with channel-trust provenance (only `user_input` is
/// trusted; sanitize-first, flag-not-block, Go reason wording).
pub fn assess_crisis_with_provenance(
    data: &Value,
    channel: Channel,
) -> CrisisAssessmentWithProvenance {
    let owned = text_from_data(data);
    let sanitized = sanitize_for_assessment(&owned);
    let mut patched = data.clone();
    patched["text"] = json!(sanitized.content);
    let assessment = assess_crisis(&patched);
    let trusted = channel == Channel::UserInput;
    let mut flagged = !sanitized.clean;
    let mut reason = if flagged {
        Some(sanitized.reason.clone().unwrap_or_default())
    } else {
        None
    };
    if !trusted && !flagged {
        flagged = true;
        reason = Some(format!("untrusted channel: {}", channel.as_str()));
    }
    CrisisAssessmentWithProvenance {
        assessment,
        channel,
        trusted,
        flagged,
        flag_reason: reason,
    }
}

/// Generate a safe response script for the level (Go wording).
pub fn generate_crisis_response(level: CrisisLevel) -> String {
    match level {
        CrisisLevel::Red | CrisisLevel::Black => "I'm concerned about your safety. Please reach out to emergency services or a crisis hotline right now. You are not alone, and help is available.".to_string(),
        CrisisLevel::Orange => "It sounds like you're going through something really difficult. I'm here with you. Would you like help finding crisis support resources?".to_string(),
        CrisisLevel::Yellow => "I hear that things feel heavy right now. Let's take this one step at a time — would some coping strategies help?".to_string(),
        CrisisLevel::Green => "Thanks for sharing how you're feeling. I'm here whenever you want to talk.".to_string(),
    }
}
