//! Markmap Skeleton Generator
//! Generates consistent .mmd files for the grok-cli project documentation.

use std::fs;
use std::path::Path;

fn main() {
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
        fs::write(&path, content).expect(&format!("Failed to write {}", filename));
        println!("Generated: {}", path.display());
    }

    println!("\n✅ All 12 markmap skeletons generated in {}", out_dir);
}

// Consistent skeleton template style (matching main.mmd)
fn skeleton(title: &str, sections: Vec<(&str, Vec<&str>)>) -> String {
    let mut out = format!("# grok-cli · {}\n\n", title);

    for (heading, items) in sections {
        out.push_str(&format!("## {}\n", heading));
        for item in items {
            out.push_str(&format!("- {}\n", item));
        }
        out.push_str("\n");
    }

    out.push_str("## Dependencies\n- (to be filled)\n\n");
    out.push_str("## Notes\n- (to be filled)\n");
    out
}

fn overview_content() -> String {
    skeleton("Overview", vec![
        ("Entry Point", vec!["`main.rs`", "`lib.rs`"]),
        ("High-Level Architecture", vec!["CLI", "Agent", "RAG", "Tools", "Memory"]),
    ])
}

fn cli_content() -> String {
    skeleton("CLI", vec![
        ("App Entry", vec!["`cli/app.rs`", "`cli/mod.rs`"]),
        ("Commands", vec![
            "chat, code, config, tools, skills, setup...",
            "`cli/commands/*.rs`"
        ]),
        ("Interactive UI", vec!["`display/`"]),
    ])
}

fn agent_content() -> String {
    skeleton("Agent System", vec![
        ("Core", vec!["`agent/manager.rs`", "`agent/mod.rs`"]),
        ("Planner & Router", vec!["`agent/planner.rs`", "`agent/router.rs`"]),
        ("Explorer", vec!["`agent/explorer/`"]),
        ("Prompts", vec!["`agent/prompts/`"]),
    ])
}

fn rag_content() -> String {
    skeleton("RAG System", vec![
        ("Core", vec!["`rag/mod.rs`", "`rag/api.rs`"]),
        ("Graph", vec!["`rag/graph/`"]),
        ("Retrieval", vec!["`rag/retrieval/`"]),
        ("Parser", vec!["`rag/parser/`"]),
    ])
}

fn memory_content() -> String {
    skeleton("Memory System", vec![
        ("Layers", vec![
            "`memory/short_term.rs`",
            "`memory/long_term.rs`",
            "`memory/working.rs`",
            "`memory/episodic.rs`"
        ]),
        ("Compression & Archive", vec!["`memory/context_compressor.rs`", "`memory/context_archive.rs`"]),
    ])
}

fn tools_content() -> String {
    skeleton("Tools", vec![
        ("Registry & Dispatch", vec!["`tools/registry.rs`", "`tools/mod.rs`"]),
        ("Tool Categories", vec![
            "file, shell, web, mcp, agent, memory, skills...",
            "`tools/*_tools.rs`"
        ]),
        ("Sandbox & Arbitration", vec!["`tools/sandbox.rs`", "`tools/tool_arbitration.rs`"]),
    ])
}

fn engine_content() -> String {
    skeleton("Execution Engine", vec![
        ("Core Loop", vec!["`engine/mod.rs`", "`engine/execution.rs`"]),
        ("Beliefs & Arbitration", vec!["`engine/beliefs.rs`", "`engine/arbitration.rs`"]),
        ("Planner & Correction", vec!["`engine/planner.rs`", "`engine/correction.rs`"]),
    ])
}

fn router_content() -> String {
    skeleton("Router", vec![
        ("App Router", vec!["`router/app_router.rs`"]),
        ("Backends", vec!["`router/backends/`"]),
        ("CPU & Cost Routing", vec!["`router/cpu_router.rs`", "`optimizer/cost_router.rs`"]),
    ])
}

fn config_context_content() -> String {
    skeleton("Config & Context", vec![
        ("Config", vec!["`config/mod.rs`"]),
        ("Context Engine", vec![
            "`context/engine.rs`",
            "`context/session_manager.rs`",
            "`context/prompt_builder.rs`"
        ]),
    ])
}

fn mcp_acp_content() -> String {
    skeleton("MCP & ACP", vec![
        ("MCP", vec!["`mcp/client.rs`", "`mcp/protocol.rs`"]),
        ("ACP", vec![
            "`acp/mod.rs`",
            "`acp/protocol.rs`",
            "`acp/tools.rs`",
            "`acp/elicitation.rs`"
        ]),
    ])
}

fn safety_security_content() -> String {
    skeleton("Safety & Security", vec![
        ("Safety", vec!["`safety/mod.rs`", "`safety/diff_validator.rs`", "`safety/intent_validator.rs`"]),
        ("Security", vec!["`security/audit.rs`", "`security/mod.rs`"]),
    ])
}

fn utils_display_content() -> String {
    skeleton("Utils & Display", vec![
        ("Utils", vec!["`utils/` (auth, client, history, telemetry...)"]),
        ("Display", vec!["`display/` (banner, terminal, interactive components)"]),
        ("Terminal", vec!["`terminal/`"]),
    ])
}