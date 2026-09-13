//! Sleepwalker Protocol — emotional-state detection with channel provenance.
//!
//! Mirrors the canonical indicator lexicon and the Go port's provenance
//! semantics: only `user_input` is trusted; every other channel is flagged.
//! Sanitize-first: assessed text is sanitized and injection findings flag
//! provenance without blocking analysis.

use crate::dto::{EmotionalState, EmotionalStateWithProvenance};
use crate::promptdefense::sanitize_for_assessment;
use crate::types::Channel;
use std::collections::BTreeMap;

/// One emotional signal category from the canonical lexicon.
struct EmotionalIndicator {
    name: &'static str,
    category: &'static str,
    weight: f64,
    patterns: &'static [&'static str],
}

/// Canonical 8-indicator lexicon (negative + positive markers).
static EMOTIONAL_INDICATORS: &[EmotionalIndicator] = &[
    EmotionalIndicator {
        name: "crisis_speak",
        category: "crisis",
        weight: 0.4,
        patterns: &[
            "kill myself",
            "end my life",
            "suicide",
            "want to die",
            "hurt myself",
        ],
    },
    EmotionalIndicator {
        name: "depression",
        category: "depression",
        weight: 0.3,
        patterns: &["hopeless", "worthless", "empty inside", "no point"],
    },
    EmotionalIndicator {
        name: "anxiety",
        category: "anxiety",
        weight: 0.25,
        patterns: &["anxious", "panic", "overwhelmed", "can't cope", "on edge"],
    },
    EmotionalIndicator {
        name: "distress",
        category: "distress",
        weight: 0.3,
        patterns: &["distressed", "falling apart", "can't take", "breaking down"],
    },
    EmotionalIndicator {
        name: "anger",
        category: "anger",
        weight: 0.2,
        patterns: &["furious", "enraged", "so angry", "want to scream"],
    },
    EmotionalIndicator {
        name: "confusion",
        category: "confusion",
        weight: 0.15,
        patterns: &["confused", "lost", "don't understand", "disoriented"],
    },
    EmotionalIndicator {
        name: "gratitude",
        category: "positive",
        weight: -0.25,
        patterns: &["thank you", "grateful", "appreciate", "means a lot"],
    },
    EmotionalIndicator {
        name: "calm",
        category: "positive",
        weight: -0.2,
        patterns: &["feeling better", "calm", "at peace", "in control"],
    },
];

/// Dilution factor for very short texts (confidence-shrinkage heuristic from
/// the canonical/Go implementation: <5 word-chars -> x0.6, <15 -> x0.85).
fn dilution_factor(text: &str) -> f64 {
    let word_chars = text.chars().filter(|c| c.is_alphanumeric()).count();
    if word_chars < 5 {
        0.6
    } else if word_chars < 15 {
        0.85
    } else {
        1.0
    }
}

/// Analyze the emotional state of a text against the canonical lexicon.
pub fn analyze_emotional_state(text: &str) -> EmotionalState {
    let lowered = text.to_lowercase();
    let mut raw_scores: BTreeMap<String, f64> = BTreeMap::new();
    let mut indicators: Vec<String> = Vec::new();
    let mut score = 0.0_f64;
    let mut top_category = "";
    let mut top_abs = 0.0_f64;
    for indicator in EMOTIONAL_INDICATORS {
        let mut hits: Vec<&str> = Vec::new();
        for p in indicator.patterns {
            if lowered.contains(*p) {
                hits.push(p);
            }
        }
        if hits.is_empty() {
            continue;
        }
        score += indicator.weight;
        raw_scores.insert(indicator.name.to_string(), indicator.weight);
        for hit in hits {
            indicators.push(format!("{}:{}", indicator.name, hit));
        }
        if indicator.weight.abs() > top_abs {
            top_abs = indicator.weight.abs();
            top_category = indicator.category;
        }
    }
    let state: &str = if score >= 0.4 {
        "crisis"
    } else if score >= 0.2 {
        "distress"
    } else if score > 0.0 {
        top_category
    } else if score == 0.0 {
        "neutral"
    } else {
        "positive"
    };
    let confidence = (score * dilution_factor(text)).clamp(0.0, 1.0);
    EmotionalState {
        state: state.to_string(),
        confidence,
        indicators,
        raw_scores,
    }
}

/// Evaluate channel trust for a Sleepwalker assessment: only `user_input` is
/// trusted; every other channel (known or not) is flagged (canonical trust
/// model — `system` provenance is NOT trusted).
fn trust_channel(channel: Channel) -> (bool, bool, Option<String>) {
    match channel {
        Channel::UserInput => (true, false, None),
        Channel::Unknown => (false, true, Some("unknown channel provenance".to_string())),
        other => (
            false,
            true,
            Some(format!("untrusted channel provenance: {}", other.as_str())),
        ),
    }
}

/// Assess emotional state with channel-trust provenance. Sanitize-first: the
/// text is sanitized before analysis and high-risk injection findings flag the
/// provenance without blocking the assessment.
pub fn assess_emotional_state_with_provenance(
    text: &str,
    channel: Channel,
) -> EmotionalStateWithProvenance {
    let sanitized = sanitize_for_assessment(text);
    let state = analyze_emotional_state(&sanitized.content);
    let (mut trusted, mut flagged, mut reason) = trust_channel(channel);
    if sanitized.risk_level == crate::types::RiskLevel::High {
        flagged = true;
        let detail = format!(
            "injection risk: {}",
            sanitized.reason.as_deref().unwrap_or("")
        );
        reason = Some(match reason {
            Some(existing) => format!("{existing}; {detail}"),
            None => detail,
        });
        if trusted {
            trusted = false;
        }
    }
    EmotionalStateWithProvenance {
        state,
        channel,
        trusted,
        flagged,
        flag_reason: reason,
    }
}

/// Reports whether an assessed emotional state warrants an RRT Advocate
/// handoff, mirroring the canonical predicate: any negative emotional state
/// (distress, anxiety, sadness, anger, depression, confusion) or a crisis
/// classification hands off immediately; positive/neutral states do not.
pub fn requires_rrta_handoff(state: &EmotionalState) -> bool {
    matches!(
        state.state.as_str(),
        "distress" | "anxiety" | "sadness" | "anger" | "depression" | "confusion" | "crisis"
    )
}
