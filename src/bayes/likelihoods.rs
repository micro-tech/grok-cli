use std::collections::HashMap;

/// Build a likelihood map from user text.
///
/// `weight` controls the spike applied to matching intent hypotheses.
/// Use [`DEFAULT_INTENT_LIKELIHOOD_WEIGHT`] (5.0) when no config is available.
/// Higher values make the router commit to an intent more decisively on a
/// keyword match; lower values produce softer, more conservative routing.
pub fn likelihood_from_text(text: &str, weight: f32) -> HashMap<String, f32> {
    let t = text.to_lowercase();
    let mut map = HashMap::new();

    if t.contains("edit") || t.contains("fix") || t.contains("refactor") {
        map.insert("intent_edit".into(), weight);
    }
    if t.contains("run ") || t.contains("execute") || t.contains("shell") {
        map.insert("intent_shell".into(), weight);
    }
    if t.contains("search") || t.contains("google") || t.contains("web") {
        map.insert("intent_search".into(), weight);
    }
    if t.ends_with("?") || t.contains("what is") || t.contains("explain") {
        map.insert("intent_question".into(), weight);
    }

    // ambiguity / risk
    if t.contains("careful") || t.contains("don't delete") {
        map.insert("need_clarification".into(), 10.0);
        map.insert("low_confidence".into(), 5.0);
    }

    // Vagueness heuristic
    let word_count = t.split_whitespace().count();
    if word_count < 3 && !t.contains("edit") && !t.contains("run") && !t.contains("search") {
        map.insert("is_vague".into(), 8.0);
    } else if word_count > 10 {
        map.insert("is_vague".into(), 0.1);
    }

    if t == "reset_clarification" {
        map.insert("need_clarification".into(), 0.01);
        map.insert("low_confidence".into(), 0.01);
    }

    map
}

/// The compiled-in default for `weight` in [`likelihood_from_text`].
/// Matches the default value of `bayesian.intent_likelihood_weight` in config.
pub const DEFAULT_INTENT_LIKELIHOOD_WEIGHT: f32 = 5.0;

pub fn likelihood_from_model_confidence(score: f32) -> HashMap<String, f32> {
    let mut map = HashMap::new();
    if score < 0.4 {
        map.insert("low_confidence".into(), 0.9);
        map.insert("need_clarification".into(), 0.7);
    } else if score < 0.7 {
        map.insert("low_confidence".into(), 0.5);
    }
    map
}

pub fn likelihood_from_tool_failure() -> HashMap<String, f32> {
    let mut map = HashMap::new();
    map.insert("low_confidence".into(), 0.8);
    map.insert("need_clarification".into(), 0.6);
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_likelihood_from_text() {
        let w = DEFAULT_INTENT_LIKELIHOOD_WEIGHT;
        let l = likelihood_from_text("please edit this file", w);
        assert_eq!(l.get("intent_edit"), Some(&w));
        assert_eq!(l.get("intent_shell"), None);

        let l = likelihood_from_text("run the build command", w);
        assert_eq!(l.get("intent_shell"), Some(&w));

        let l = likelihood_from_text("be careful don't delete anything", w);
        assert_eq!(l.get("need_clarification"), Some(&10.0));
        assert_eq!(l.get("low_confidence"), Some(&5.0));
    }

    #[test]
    fn test_custom_weight_applied() {
        let l = likelihood_from_text("please edit this file", 9.0);
        assert_eq!(l.get("intent_edit"), Some(&9.0));
    }

    #[test]
    fn test_low_weight_still_matches() {
        let l = likelihood_from_text("search the web", 0.5);
        assert_eq!(l.get("intent_search"), Some(&0.5));
    }

    #[test]
    fn test_likelihood_from_model_confidence() {
        let l = likelihood_from_model_confidence(0.2);
        assert_eq!(l.get("low_confidence"), Some(&0.9));

        let l = likelihood_from_model_confidence(0.9);
        assert_eq!(l.get("low_confidence"), None);
    }
}
