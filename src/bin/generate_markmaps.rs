//! Markmap Skeleton Generator
//! Generates consistent .mmd files for the grok-cli project documentation.
//!
//! NOTE: The .mmd files in `.doc/markmap/` are hand-enriched after
//! generation (real file lists, descriptions, cross-links). This generator
//! only produces starting skeletons, so by default it SKIPS files that
//! already exist — re-running it will not wipe out hand-written content.
//! Pass `--force` to overwrite existing files with fresh skeletons.
//!
//! The skeleton contents below are kept in sync with the actual `src/`
//! tree. If a module gains/renames files, update the matching `*_content()`
//! function here so a fresh skeleton names real files.

use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let force = env::args().any(|a| a == "--force");
    let out_dir = ".doc/markmap";
    fs::create_dir_all(out_dir).expect("Failed to create markmap directory");

    let markmaps = vec![
        ("00-overview.mmd", overview_content()),
        ("01-cli.mmd", cli_content()),
        ("02-agent.mmd", agent_content()),
        ("03-rag.mmd", rag_content()),
        ("04-memory.mmd", memory_content()),
        ("05-tools.mmd", tools_content()),
        ("06-engine.mmd", engine_content()),
        ("07-router.mmd", router_content()),
        ("08-config-context.mmd", config_context_content()),
        ("09-mcp-acp.mmd", mcp_acp_content()),
        ("10-safety-security.mmd", safety_security_content()),
        ("11-utils-display.mmd", utils_display_content()),
    ];

    for (filename, content) in markmaps {
        let path = Path::new(out_dir).join(filename);
        if path.exists() && !force {
            println!("Skipped (exists, use --force to overwrite): {}", path.display());
            continue;
        }
        fs::write(&path, content).unwrap_or_else(|_| panic!("Failed to write {filename}"));
        println!("Generated: {}", path.display());
    }

    println!("\n✅ Markmap skeletons up to date in {}", out_dir);
    if !force {
        println!("(Existing files were left untouched. Re-run with --force to regenerate them.)");
    }
}

// Consistent skeleton template style (matching main.mmd)
fn skeleton(title: &str, sections: Vec<(&str, Vec<&str>)>) -> String {
    let mut out = format!("# grok-cli · {}\n\n", title);

    for (heading, items) in sections {
        out.push_str(&format!("## {}\n", heading));
        for item in items {
            out.push_str(&format!("- {}\n", item));
        }
        out.push('\n');
    }

    out.push_str("## Dependencies\n- (to be filled)\n\n");
    out.push_str("## Notes\n- (to be filled)\n");
    out
}

fn overview_content() -> String {
    skeleton(
        "Overview",
        vec![
            ("Entry Point", vec!["`main.rs`", "`lib.rs`"]),
            (
                "High-Level Architecture",
                vec![
                    "CLI",
                    "Agent",
                    "RAG",
                    "Tools",
                    "Memory",
                    "Engine",
                    "Router",
                    "HOH (Harness-of-Harnesses)",
                    "Bayesian",
                    "Skills",
                    "Workflow",
                ],
            ),
        ],
    )
}

fn cli_content() -> String {
    skeleton(
        "CLI",
        vec![
            (
                "App Entry",
                vec!["`cli/app.rs`", "`cli/mod.rs`", "`cli/approval.rs`"],
            ),
            (
                "Commands",
                vec![
                    "chat, code, config, tools, skills, setup, update...",
                    "`cli/commands/*.rs`",
                ],
            ),
            ("Interactive UI", vec!["`display/`", "`terminal/`"]),
        ],
    )
}

fn agent_content() -> String {
    skeleton(
        "Agent System",
        vec![
            (
                "Core",
                vec![
                    "`agent/manager.rs`",
                    "`agent/mod.rs`",
                    "`agent/mode.rs`",
                    "`agent/activity.rs`",
                    "`agent/config.rs`",
                    "`agent/loader.rs`",
                    "`agent/simulator.rs`",
                ],
            ),
            (
                "Planner & Router",
                vec![
                    "`agent/planner.rs`",
                    "`agent/router.rs`",
                    "`agent/cost_router.rs`",
                ],
            ),
            (
                "Multi-Agent",
                vec!["`agent/message_bus.rs`", "`agent/explorer/`"],
            ),
            ("Prompts", vec!["`agent/prompts/`"]),
        ],
    )
}

fn rag_content() -> String {
    skeleton(
        "RAG System",
        vec![
            (
                "Core",
                vec![
                    "`rag/mod.rs`",
                    "`rag/api.rs`",
                    "`rag/config.rs`",
                    "`rag/debug.rs`",
                ],
            ),
            ("Graph", vec!["`rag/graph/`"]),
            ("Index", vec!["`rag/index/` — BM25 + chunk indexing"]),
            ("Retrieval", vec!["`rag/retrieval/`"]),
            ("Parser", vec!["`rag/parser/`"]),
            (
                "Integrations",
                vec![
                    "`rag/acp_integration.rs`",
                    "`rag/dna_integration.rs`",
                    "`rag/compression.rs`",
                    "`rag/persistence.rs`",
                ],
            ),
        ],
    )
}

fn memory_content() -> String {
    skeleton(
        "Memory System",
        vec![
            (
                "Layers",
                vec![
                    "`memory/short_term.rs`",
                    "`memory/long_term.rs`",
                    "`memory/working.rs`",
                    "`memory/episodic.rs`",
                    "`memory/memory_manager.rs`",
                ],
            ),
            (
                "Specialized Memory",
                vec!["`memory/skill_memory.rs`", "`memory/tool_memory.rs`"],
            ),
            (
                "Compression & Archive",
                vec![
                    "`memory/context_compressor.rs`",
                    "`memory/context_archive.rs`",
                    "`memory/compaction.rs`",
                    "`memory/replace_slot.rs`",
                    "`memory/types.rs`",
                ],
            ),
        ],
    )
}

fn tools_content() -> String {
    skeleton(
        "Tools",
        vec![
            (
                "Registry & Dispatch",
                vec!["`tools/registry.rs`", "`tools/mod.rs`"],
            ),
            (
                "Tool Categories",
                vec![
                    "file, shell, web, mcp, agent, memory, skills, lsp, vision, okf...",
                    "`tools/*_tools.rs`",
                ],
            ),
            (
                "Sandbox & Arbitration",
                vec!["`tools/sandbox.rs`", "`tools/tool_arbitration.rs`"],
            ),
        ],
    )
}

fn engine_content() -> String {
    skeleton(
        "Execution Engine",
        vec![
            (
                "Core Loop",
                vec!["`engine/mod.rs`", "`engine/execution.rs`", "`engine/state.rs`"],
            ),
            (
                "Beliefs & Arbitration",
                vec!["`engine/beliefs.rs`", "`engine/arbitration.rs`"],
            ),
            (
                "Planner & Correction",
                vec!["`engine/planner.rs`", "`engine/correction.rs`"],
            ),
            (
                "Integration Points",
                vec!["`engine/memory_bridge.rs`", "`engine/observability.rs`"],
            ),
        ],
    )
}

fn router_content() -> String {
    skeleton(
        "Router",
        vec![
            (
                "App Router",
                vec![
                    "`router/app_router.rs`",
                    "`router/backend.rs`",
                    "`router/mod.rs`",
                ],
            ),
            ("Backends", vec!["`router/backends/`"]),
            (
                "CPU & Cost Routing",
                vec!["`router/cpu_router.rs`", "`optimizer/cost_router.rs`"],
            ),
            (
                "Core Types",
                vec!["`router/request.rs`", "`router/response.rs`", "`router/router_error.rs`"],
            ),
        ],
    )
}

fn config_context_content() -> String {
    skeleton(
        "Config & Context",
        vec![
            (
                "Config",
                vec!["`config/mod.rs`", "`config/*.rs` (model, memory, bayesian, okf, ...)"],
            ),
            (
                "Context Engine",
                vec![
                    "`context/engine.rs`",
                    "`context/session_manager.rs`",
                    "`context/prompt_builder.rs`",
                    "`context/prompt_applicator.rs`",
                    "`context/tool_optimizer.rs`",
                    "`context/belief_state.rs`",
                    "`context/agent_rules.rs`",
                ],
            ),
        ],
    )
}

fn mcp_acp_content() -> String {
    skeleton(
        "MCP & ACP",
        vec![
            (
                "MCP",
                vec![
                    "`mcp/client.rs`",
                    "`mcp/protocol.rs`",
                    "`mcp/config.rs`",
                    "`mcp/mod.rs`",
                ],
            ),
            (
                "ACP",
                vec![
                    "`acp/mod.rs`",
                    "`acp/protocol.rs`",
                    "`acp/tools.rs`",
                    "`acp/elicitation.rs`",
                    "`acp/handlers.rs`",
                    "`acp/handoff.rs`",
                    "`acp/slash_commands.rs`",
                    "`acp/security.rs`",
                    "`acp/status_bar.rs`",
                    "`acp/mcp_bridge.rs`",
                    "`acp/chat_turn.rs`",
                    "`acp/context_trim.rs`",
                ],
            ),
        ],
    )
}

fn safety_security_content() -> String {
    skeleton(
        "Safety & Security",
        vec![
            (
                "Safety",
                vec![
                    "`safety/mod.rs`",
                    "`safety/diff_validator.rs`",
                    "`safety/intent_validator.rs`",
                    "`safety/pre_write_hook.rs`",
                    "`safety/suspicious_write_guard.rs`",
                    "`safety/dna_safety.rs`",
                    "`safety/dry_run.rs`",
                    "`safety/tool_health_monitor.rs`",
                ],
            ),
            ("Security", vec!["`security/audit.rs`", "`security/mod.rs`"]),
        ],
    )
}

fn utils_display_content() -> String {
    skeleton(
        "Utils & Display",
        vec![
            (
                "Utils",
                vec!["`utils/` (auth, client, history, telemetry...)"],
            ),
            (
                "Display",
                vec!["`display/` (banner, terminal, interactive components)"],
            ),
            ("Terminal", vec!["`terminal/`"]),
            (
                "HOH",
                vec!["`hoh/` — Harness-of-Harnesses (~97 modules, see 1.mmd)"],
            ),
            (
                "Hooks / Knowledge / Workflow",
                vec!["`hooks/`", "`knowledge/`", "`workflow/`", "`task_graph/`"],
            ),
        ],
    )
}
