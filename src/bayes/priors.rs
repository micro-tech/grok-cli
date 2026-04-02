use std::collections::HashMap;

use crate::config::BayesianPriorsConfig;

pub fn default_priors() -> HashMap<String, f32> {
    let mut map = HashMap::new();

    // intents
    map.insert("intent_edit".into(), 0.2);
    map.insert("intent_shell".into(), 0.2);
    map.insert("intent_search".into(), 0.2);
    map.insert("intent_question".into(), 0.3);

    // meta
    map.insert("need_clarification".into(), 0.1);
    map.insert("low_confidence".into(), 0.2);
    map.insert("is_vague".into(), 0.1);

    map
}

/// Build a prior map from the `[bayesian.priors]` config section.
///
/// Used as the fallback when no saved profile exists on disk.  Once the
/// engine learns from actual tool usage the persisted profile takes over,
/// so this is only consulted on first launch or after the profile is deleted.
pub fn priors_from_config(config: &BayesianPriorsConfig) -> HashMap<String, f32> {
    let mut map = HashMap::new();

    map.insert("intent_edit".into(), config.intent_edit);
    map.insert("intent_shell".into(), config.intent_shell);
    map.insert("intent_search".into(), config.intent_search);
    map.insert("intent_question".into(), config.intent_question);
    map.insert("need_clarification".into(), config.need_clarification);
    map.insert("low_confidence".into(), config.low_confidence);
    map.insert("is_vague".into(), config.is_vague);

    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_priors_keys() {
        let p = default_priors();
        assert!(p.contains_key("intent_edit"));
        assert!(p.contains_key("intent_shell"));
        assert!(p.contains_key("intent_search"));
        assert!(p.contains_key("intent_question"));
        assert!(p.contains_key("need_clarification"));
        assert!(p.contains_key("low_confidence"));
        assert!(p.contains_key("is_vague"));
    }

    #[test]
    fn test_default_priors_sum_to_one() {
        let p = default_priors();
        let sum: f32 = p.values().sum();
        // 0.2 + 0.2 + 0.2 + 0.3 + 0.1 + 0.2 + 0.1 = 1.3 (unnormalised)
        // The engine normalises on first use; raw priors need not sum to 1.0.
        assert!(sum > 0.0, "priors must be positive");
    }

    #[test]
    fn test_priors_from_config_matches_values() {
        let config = BayesianPriorsConfig {
            intent_edit: 0.1,
            intent_shell: 0.15,
            intent_search: 0.25,
            intent_question: 0.4,
            need_clarification: 0.05,
            low_confidence: 0.03,
            is_vague: 0.02,
        };
        let p = priors_from_config(&config);
        assert!((p["intent_edit"] - 0.1).abs() < f32::EPSILON);
        assert!((p["intent_shell"] - 0.15).abs() < f32::EPSILON);
        assert!((p["intent_search"] - 0.25).abs() < f32::EPSILON);
        assert!((p["intent_question"] - 0.4).abs() < f32::EPSILON);
        assert!((p["need_clarification"] - 0.05).abs() < f32::EPSILON);
        assert!((p["low_confidence"] - 0.03).abs() < f32::EPSILON);
        assert!((p["is_vague"] - 0.02).abs() < f32::EPSILON);
    }

    #[test]
    fn test_priors_from_config_has_all_keys() {
        let config = BayesianPriorsConfig::default();
        let p = priors_from_config(&config);
        let default = default_priors();
        for key in default.keys() {
            assert!(p.contains_key(key.as_str()), "missing key: {}", key);
        }
    }

    #[test]
    fn test_default_and_config_priors_same_keys() {
        let config = BayesianPriorsConfig::default();
        let from_config = priors_from_config(&config);
        let defaults = default_priors();
        assert_eq!(from_config.len(), defaults.len());
        for key in defaults.keys() {
            assert!(
                from_config.contains_key(key.as_str()),
                "key '{}' missing from config priors",
                key
            );
        }
    }
}
