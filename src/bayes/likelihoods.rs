use std::collections::HashMap;

pub fn likelihood_from_text(text: &str) -> HashMap<String, f32> {
    let t = text.to_lowercase();
    let mut map = HashMap::new();

    if t.contains("edit") || t.contains("fix") || t.contains("refactor") {
        map.insert("intent_edit".into(), 5.0);
    }
    if t.contains("run ") || t.contains("execute") || t.contains("shell") {
        map.insert("intent_shell".into(), 5.0);
    }
    if t.contains("search") || t.contains("google") || t.contains("web") {
        map.insert("intent_search".into(), 5.0);
    }
    if t.ends_with("?") || t.contains("what is") || t.contains("explain") {
        map.insert("intent_question".into(), 5.0);
    }

    // ambiguity / risk
    if t.contains("careful") || t.contains("don’t delete") || t.contains("don't delete") {
        map.insert("need_clarification".into(), 10.0);
        map.insert("low_confidence".into(), 5.0);
    }

    if t == "reset_clarification" {
        map.insert("need_clarification".into(), 0.01);
        map.insert("low_confidence".into(), 0.01);
    }

    map
}

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
        let l = likelihood_from_text("please edit this file");
        assert_eq!(l.get("intent_edit"), Some(&5.0));
        assert_eq!(l.get("intent_shell"), None);

        let l = likelihood_from_text("run the build command");
        assert_eq!(l.get("intent_shell"), Some(&5.0));

        let l = likelihood_from_text("be careful don't delete anything");
        assert_eq!(l.get("need_clarification"), Some(&10.0));
        assert_eq!(l.get("low_confidence"), Some(&5.0));
    }

    #[test]
    fn test_likelihood_from_model_confidence() {
        let l = likelihood_from_model_confidence(0.2);
        assert_eq!(l.get("low_confidence"), Some(&0.9));

        let l = likelihood_from_model_confidence(0.9);
        assert_eq!(l.get("low_confidence"), None);
    }
}
