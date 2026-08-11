use std::collections::HashMap;

/// Build a likelihood map from user text.
///
/// `weight` controls the spike applied to matching intent hypotheses.
/// Use [`DEFAULT_INTENT_LIKELIHOOD_WEIGHT`] (5.0) when no config is available.
/// Higher values make the router commit to an intent more decisively on a
/// keyword match; lower values produce softer, more conservative routing.
pub fn likelihood_from_text(text: &str, weight: f32) -> HashMap<String, f32> {
    // Task 268.2: Use the zero-cost perf guard (only active under GROK_PERF=1)
    crate::perf_guard!("bayes.likelihood_from_text");

    // Cheap short-circuit for trivial/empty input (Task 268 optimization)
    let trimmed = text.trim();
    if trimmed.len() < 3 {
        return HashMap::new();
    }

    // Use ascii lowercase (faster for typical command text, still correct for our keywords)
    let t = trimmed.to_ascii_lowercase();
    // Pre-allocate to avoid reallocs in the common 0-4 entry case
    let mut map = HashMap::with_capacity(6);

    if t.contains("edit") || t.contains("fix") || t.contains("refactor") || t.contains("change") {
        map.insert("intent_edit".into(), weight);
    }
    if t.contains("run ") || t.contains("execute") || t.contains("shell") || t.contains("cmd") {
        map.insert("intent_shell".into(), weight);
    }
    if t.contains("search") || t.contains("google") || t.contains("web") || t.contains("fetch") {
        map.insert("intent_search".into(), weight);
    }
    if t.ends_with('?') || t.contains("what is") || t.contains("explain") || t.contains("how ") {
        map.insert("intent_question".into(), weight);
    }

    // ambiguity / risk
    //
    // "careful" / "don't delete" are strong risk signals, so we give
    // `need_clarification` a likelihood of 15.0.
    if t.contains("careful") || t.contains("don't delete") || (t.contains("delete") && t.contains("don't")) {
        map.insert("need_clarification".into(), 15.0);
        map.insert("low_confidence".into(), 5.0);
    }

    // Vagueness heuristic — only set vague if we didn't detect a strong intent
    let word_count = t.split_whitespace().count();
    let has_strong_intent = map.keys().any(|k| k.starts_with("intent_"));
    if word_count < 3 && !has_strong_intent {
        map.insert("is_vague".into(), 8.0);
    } else if word_count > 12 {
        map.insert("is_vague".into(), 0.05);
    }

    if t == "reset_clarification" {
        map.clear();
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
        // need_clarification likelihood is 15.0 (raised from 10.0 so it
        // exceeds the 0.4 clarification threshold after normalisation).
        assert_eq!(l.get("need_clarification"), Some(&15.0));
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
    fn test_new_keywords_and_vague_logic() {
        let w = DEFAULT_INTENT_LIKELIHOOD_WEIGHT;

        // New keywords
        let l = likelihood_from_text("please change this code", w);
        assert!(l.contains_key("intent_edit"));

        let l = likelihood_from_text("run cmd to build", w);
        assert!(l.contains_key("intent_shell"));

        let l = likelihood_from_text("fetch the docs", w);
        assert!(l.contains_key("intent_search"));

        let l = likelihood_from_text("how do I do this?", w);
        assert!(l.contains_key("intent_question"));

        // Vague detection should not fire when strong intent is present
        let l = likelihood_from_text("edit foo", w);
        assert!(!l.contains_key("is_vague"));

        // True vague short input
        let l = likelihood_from_text("ok", w);
        assert_eq!(l.get("is_vague"), Some(&8.0));

        // Long input should get low vague score
        let long = "this is a very long sentence that has many words and should trigger the long text path";
        let l = likelihood_from_text(long, w);
        assert_eq!(l.get("is_vague"), Some(&0.05));
    }

    #[test]
    fn test_likelihood_from_model_confidence() {
        let l = likelihood_from_model_confidence(0.2);
        assert_eq!(l.get("low_confidence"), Some(&0.9));

        let l = likelihood_from_model_confidence(0.9);
        assert_eq!(l.get("low_confidence"), None);
    }
}
