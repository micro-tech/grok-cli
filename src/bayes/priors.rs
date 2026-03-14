use std::collections::HashMap;

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

    map
}
