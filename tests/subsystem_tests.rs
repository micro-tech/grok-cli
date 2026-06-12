//! Subsystem integration tests
//!
//! Covers:
//!   - Memory subsystem (long-term store)      — tests 1–5
//!   - Bayesian router / engine                 — tests 6–9
//!   - Config loading (AcpConfig, ThinkingMode) — tests 10–14
//!   - Tool registry / arbitration              — tests 15–20
//!
//! ## Design constraints
//! - **No network** — all tests run fully offline.
//! - **tempfile isolation** — each test that touches disk gets its own temp dir.
//! - **Deterministic** — no random seeds, no timing dependencies.

// ── imports ───────────────────────────────────────────────────────────────────

use grok_cli::{
    bayes::BayesianEngine,
    config::{AcpConfig, BayesianConfig, ThinkingMode},
    memory::{long_term::LongTermMemory, types::MemorySource},
    tools::{
        get_full_tool_definitions, get_tool_definitions, save_memory,
        tool_arbitration::{ArbitrationDecision, arbitrate_tool_call},
    },
};
use serde_json::json;

// ─────────────────────────────────────────────────────────────────────────────
// Memory subsystem — tests 1-5
// ─────────────────────────────────────────────────────────────────────────────

/// Test 1 — save a fact, reload from the same temp dir, fact persists.
#[test]
fn mem_save_fact_persists_across_reload() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = LongTermMemory::load_or_create_at(dir.path()).expect("load_or_create_at");

    let id = mem
        .save_fact(
            "user prefers dark mode",
            MemorySource::User,
            vec!["ui".into()],
        )
        .expect("save_fact");
    assert!(!id.is_empty(), "returned id must not be empty");

    // Reload from the same directory — fact must survive round-trip.
    let mem2 = LongTermMemory::load_or_create_at(dir.path()).expect("reload");
    assert_eq!(mem2.len(), 1, "reloaded store must have exactly 1 entry");
    assert_eq!(mem2.all()[0].fact, "user prefers dark mode");
    assert_eq!(mem2.all()[0].id, id, "id must be stable after reload");
}

/// Test 2 — saving the same fact twice is a no-op (deduplication).
#[test]
fn mem_duplicate_fact_is_not_added() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = LongTermMemory::load_or_create_at(dir.path()).expect("load_or_create_at");

    let id1 = mem
        .save_fact("prefer tabs over spaces", MemorySource::User, vec![])
        .expect("first save");
    let id2 = mem
        .save_fact("prefer tabs over spaces", MemorySource::User, vec![])
        .expect("second save (duplicate)");

    assert_eq!(id1, id2, "duplicate returns the same existing id");
    assert_eq!(mem.len(), 1, "count must stay at 1 after duplicate save");
}

/// Test 3 — `search` returns entries whose fact text matches the query term.
#[test]
fn mem_search_returns_matching_entries() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = LongTermMemory::load_or_create_at(dir.path()).expect("load_or_create_at");

    mem.save_fact("user prefers dark mode", MemorySource::User, vec![])
        .expect("save 1");
    mem.save_fact("project uses Rust 2024 edition", MemorySource::User, vec![])
        .expect("save 2");
    mem.save_fact(
        "always run clippy before commit",
        MemorySource::User,
        vec![],
    )
    .expect("save 3");

    let results = mem.search("rust");
    assert_eq!(results.len(), 1, "only one fact contains 'rust'");
    assert!(
        results[0].fact.to_lowercase().contains("rust"),
        "matched fact must contain the query term"
    );

    let no_match = mem.search("xyzzy_no_match");
    assert!(no_match.is_empty(), "non-matching query returns empty vec");
}

/// Test 4 — `to_prompt_section` returns a non-empty string after at least one
///          fact has been saved.
#[test]
fn mem_to_prompt_section_non_empty_after_save() {
    let dir = tempfile::tempdir().expect("tempdir");
    let mut mem = LongTermMemory::load_or_create_at(dir.path()).expect("load_or_create_at");

    // Before any saves the section should be empty.
    assert!(
        mem.to_prompt_section(10).is_empty(),
        "prompt section must be empty when no facts exist"
    );

    mem.save_fact(
        "always add error checking",
        MemorySource::User,
        vec!["quality".into()],
    )
    .expect("save_fact");

    let section = mem.to_prompt_section(10);
    assert!(
        !section.is_empty(),
        "prompt section must be non-empty after saving a fact"
    );
    assert!(
        section.contains("always add error checking"),
        "prompt section must contain the saved fact text"
    );
}

/// Test 5 — `save_memory("")` (via the tool shim) returns `Err`.
#[test]
fn mem_save_memory_empty_string_is_err() {
    let result = save_memory("");
    assert!(result.is_err(), "empty fact string must return Err");

    let msg = result.unwrap_err().to_string();
    assert!(
        msg.contains("must not be empty") || msg.contains("empty"),
        "error message must mention emptiness, got: {msg}"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Bayesian engine — tests 6-9
// ─────────────────────────────────────────────────────────────────────────────

/// Test 6 — `new_with_default_priors()` creates an engine with non-zero priors.
#[test]
fn bayes_default_priors_are_non_empty() {
    let engine = BayesianEngine::new_with_default_priors();
    // Intent probabilities must be positive after normalization.
    assert!(
        engine.probability("intent_question") > 0.0,
        "intent_question prior must be > 0"
    );
    assert!(
        engine.probability("intent_edit") > 0.0,
        "intent_edit prior must be > 0"
    );
    assert!(
        engine.probability("intent_shell") > 0.0,
        "intent_shell prior must be > 0"
    );
    assert!(
        engine.probability("intent_search") > 0.0,
        "intent_search prior must be > 0"
    );
}

/// Test 7 — `new_with_config(&BayesianConfig::default())` constructs successfully.
#[test]
fn bayes_new_with_config_default_succeeds() {
    let cfg = BayesianConfig::default();
    let engine = BayesianEngine::new_with_config(&cfg);
    // Sanity: the engine has a best intent (prior distribution is non-trivial).
    assert!(
        engine.best_intent().is_some(),
        "engine built from default config must have a best intent"
    );
}

/// Test 8 — the clarification threshold matches the compiled-in default (0.4).
#[test]
fn bayes_clarification_threshold_matches_default() {
    let engine = BayesianEngine::new_with_default_priors();
    let threshold = engine.clarification_threshold();
    assert!(
        threshold > 0.0,
        "clarification_threshold must be > 0.0, got {threshold}"
    );
    // The default is 0.4; allow a small floating-point epsilon.
    assert!(
        (threshold - 0.4_f32).abs() < 1e-6,
        "clarification_threshold default must be ~0.4, got {threshold}"
    );
}

/// Test 9 — after `update_from_text("edit the config file")` the `intent_edit`
///          probability increases and becomes the top intent.
#[test]
fn bayes_update_from_text_shifts_intent_edit() {
    let mut engine = BayesianEngine::new_with_default_priors();

    // Baseline: intent_question starts as the dominant prior.
    let before = engine.probability("intent_edit");

    engine.update_from_text("edit the config file");

    let after = engine.probability("intent_edit");
    assert!(
        after > before,
        "intent_edit probability must increase after 'edit' keyword, \
         before={before} after={after}"
    );

    // The best intent must now be intent_edit.
    assert_eq!(
        engine.best_intent(),
        Some("intent_edit".to_string()),
        "best_intent must be intent_edit after an 'edit' update"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Config — tests 10-14
// ─────────────────────────────────────────────────────────────────────────────

/// Test 10 — `AcpConfig::default()` has sane field values.
#[test]
fn config_acp_default_has_sane_fields() {
    let cfg = AcpConfig::default();

    assert!(
        cfg.max_context_tokens > 0,
        "max_context_tokens must be > 0, got {}",
        cfg.max_context_tokens
    );
    assert_eq!(
        cfg.thinking_mode,
        ThinkingMode::Off,
        "thinking_mode must default to Off"
    );
    assert!(
        cfg.max_tool_loop_iterations > 0,
        "max_tool_loop_iterations must be > 0"
    );
    assert!(
        cfg.max_history_messages > 0,
        "max_history_messages must be > 0"
    );
}

/// Test 11 — `Config` struct is constructible and its `acp` field is accessible.
#[test]
fn config_struct_is_constructible() {
    use grok_cli::config::Config;
    let cfg = Config::default();
    // Spot-check a few top-level fields.
    assert!(cfg.default_max_tokens > 0, "default_max_tokens must be > 0");
    assert!(
        !cfg.acp.bind_host.is_empty(),
        "acp.bind_host must not be empty"
    );
}

/// Test 12 — `ThinkingMode` serialises to and deserialises from the expected
///           snake_case strings.
#[test]
fn config_thinking_mode_serde_round_trip() {
    // Serialise
    let off_json = serde_json::to_string(&ThinkingMode::Off).expect("ser Off");
    let low_json = serde_json::to_string(&ThinkingMode::Low).expect("ser Low");
    let high_json = serde_json::to_string(&ThinkingMode::High).expect("ser High");

    assert_eq!(off_json, r#""off""#, "Off must serialise to \"off\"");
    assert_eq!(low_json, r#""low""#, "Low must serialise to \"low\"");
    assert_eq!(high_json, r#""high""#, "High must serialise to \"high\"");

    // Deserialise back.
    let off: ThinkingMode = serde_json::from_str(&off_json).expect("de Off");
    let low: ThinkingMode = serde_json::from_str(&low_json).expect("de Low");
    let high: ThinkingMode = serde_json::from_str(&high_json).expect("de High");

    assert_eq!(off, ThinkingMode::Off);
    assert_eq!(low, ThinkingMode::Low);
    assert_eq!(high, ThinkingMode::High);
}

/// Test 13 — `ThinkingMode::from_str_ci` is case-insensitive.
#[test]
fn config_thinking_mode_from_str_ci() {
    assert_eq!(
        ThinkingMode::from_str_ci("HIGH"),
        Some(ThinkingMode::High),
        "\"HIGH\" must parse to High"
    );
    assert_eq!(
        ThinkingMode::from_str_ci("high"),
        Some(ThinkingMode::High),
        "\"high\" must parse to High"
    );
    assert_eq!(
        ThinkingMode::from_str_ci("Low"),
        Some(ThinkingMode::Low),
        "\"Low\" must parse to Low"
    );
    assert_eq!(
        ThinkingMode::from_str_ci("OFF"),
        Some(ThinkingMode::Off),
        "\"OFF\" must parse to Off"
    );
    assert_eq!(
        ThinkingMode::from_str_ci("unknown_value"),
        None,
        "unknown string must return None"
    );
}

/// Test 14 — `ThinkingMode::as_api_str` returns the correct values.
#[test]
fn config_thinking_mode_as_api_str() {
    assert_eq!(
        ThinkingMode::Off.as_api_str(),
        None,
        "Off must map to None (field omitted from API request)"
    );
    assert_eq!(
        ThinkingMode::Low.as_api_str(),
        Some("low"),
        "Low must map to Some(\"low\")"
    );
    assert_eq!(
        ThinkingMode::High.as_api_str(),
        Some("high"),
        "High must map to Some(\"high\")"
    );
}

// ─────────────────────────────────────────────────────────────────────────────
// Tool registry — tests 15-20
// ─────────────────────────────────────────────────────────────────────────────

/// Test 15 — `get_tool_definitions()` returns a non-empty list that includes
///           the core tools.
#[test]
fn registry_get_tool_definitions_contains_core_tools() {
    let defs = get_tool_definitions();

    assert!(!defs.is_empty(), "tool definition list must not be empty");

    let required = ["read_file", "task_get", "task_create", "task_update"];
    for name in &required {
        assert!(
            defs.contains(name),
            "tool definitions must include \"{name}\""
        );
    }
}

/// Test 16 — every entry in `get_full_tool_definitions()` has the expected
///           schema shape: `type`, `function.name`, `function.description`.
#[test]
fn registry_full_tool_definitions_have_correct_shape() {
    let full = get_full_tool_definitions();
    assert!(
        !full.is_empty(),
        "full tool definitions list must not be empty"
    );

    for def in &full {
        let type_field = def.get("type");
        assert!(
            type_field.is_some(),
            "each definition must have a \"type\" field"
        );

        let func = def
            .get("function")
            .expect("each definition must have a \"function\" object");

        assert!(
            func.get("name").and_then(|v| v.as_str()).is_some(),
            "function.name must be a non-null string in every definition, \
             got: {def}"
        );
        assert!(
            func.get("description").and_then(|v| v.as_str()).is_some(),
            "function.description must be a non-null string in every definition, \
             got: {def}"
        );
    }
}

/// Test 17 — arbitration: well-known tools with all required args are accepted;
///           unknown tools are rejected.
///           (Combines the check from the original task description.)
#[test]
fn arbitration_known_tool_passes_and_unknown_tool_rejects() {
    // Known tool with valid args → Execute.
    let decision =
        arbitrate_tool_call("read_file", &json!({"path": "src/main.rs"})).expect("arbitrate");
    assert!(
        matches!(decision, ArbitrationDecision::Execute { .. }),
        "read_file with a valid path must yield Execute"
    );

    // Unknown tool → Reject.
    let decision = arbitrate_tool_call("unknown_tool_xyz", &json!({})).expect("arbitrate");
    assert!(
        matches!(decision, ArbitrationDecision::Reject { .. }),
        "unknown tool must yield Reject"
    );
}

/// Test 18 — arbitrate "read_file" with `{"path": "foo.rs"}` → Execute.
#[test]
fn arbitration_read_file_with_path_is_execute() {
    let decision = arbitrate_tool_call("read_file", &json!({"path": "foo.rs"})).expect("arbitrate");

    match decision {
        ArbitrationDecision::Execute { name, args } => {
            assert_eq!(name, "read_file");
            assert_eq!(
                args["path"].as_str(),
                Some("foo.rs"),
                "args must preserve the supplied path"
            );
        }
        other => panic!(
            "expected Execute, got {:?}",
            match other {
                ArbitrationDecision::Reject { message } => format!("Reject({message})"),
                ArbitrationDecision::NeedMoreInfo { message, .. } => {
                    format!("NeedMoreInfo({message})")
                }
                ArbitrationDecision::Execute { .. } => unreachable!(),
            }
        ),
    }
}

/// Test 19 — arbitrate "read_file" with `{}` (missing `path`) → NeedMoreInfo.
#[test]
fn arbitration_read_file_without_path_is_need_more_info() {
    let decision = arbitrate_tool_call("read_file", &json!({})).expect("arbitrate");

    match &decision {
        ArbitrationDecision::NeedMoreInfo { missing_fields, .. } => {
            assert!(
                missing_fields.contains(&"path".to_string()),
                "missing_fields must include \"path\", got: {missing_fields:?}"
            );
        }
        ArbitrationDecision::Execute { .. } => {
            panic!("expected NeedMoreInfo but got Execute")
        }
        ArbitrationDecision::Reject { message } => {
            panic!("expected NeedMoreInfo but got Reject: {message}")
        }
    }
}

/// Test 20 — arbitrate "unknown_tool_xyz" with `{}` → Reject.
#[test]
fn arbitration_unknown_tool_is_reject() {
    let decision = arbitrate_tool_call("unknown_tool_xyz", &json!({})).expect("arbitrate");

    match &decision {
        ArbitrationDecision::Reject { message } => {
            assert!(
                message.contains("unknown_tool_xyz"),
                "reject message must name the unknown tool, got: {message}"
            );
        }
        ArbitrationDecision::Execute { .. } => {
            panic!("expected Reject but got Execute")
        }
        ArbitrationDecision::NeedMoreInfo { message, .. } => {
            panic!("expected Reject but got NeedMoreInfo: {message}")
        }
    }
}
