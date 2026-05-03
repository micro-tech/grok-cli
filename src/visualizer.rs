//! State Machine Visualizer for Grok-CLI.
//!
//! Generates a DOT (Graphviz) representation of the routing and execution
//! pipeline so users can render and inspect the system architecture.
//!
//! # Usage
//!
//! ```text
//! # Print DOT to stdout
//! grok visualize
//!
//! # Save to file and render with Graphviz
//! grok visualize --output pipeline.dot
//! dot -Tsvg pipeline.dot -o pipeline.svg
//!
//! # Or pipe directly
//! grok visualize | dot -Tpng -o pipeline.png
//! ```

use crate::config::Config;

/// Generate a DOT-format graph of the full Grok-CLI routing pipeline.
///
/// The graph captures:
/// - User input entry point
/// - Bayesian router with configured intent priors
/// - Context management layers (token trimming + smart compression)
/// - Tool loop with iteration limit
/// - LLM response output
/// - Memory subsystem (long-term, episodic, context archive)
pub fn generate_pipeline_dot(config: Option<&Config>) -> String {
    // Pull intent priors from config if available, otherwise use defaults.
    let (p_edit, p_shell, p_search, p_question) = config
        .map(|c| {
            let p = &c.bayesian.priors;
            (
                p.intent_edit,
                p.intent_shell,
                p.intent_search,
                p.intent_question,
            )
        })
        .unwrap_or((0.20, 0.20, 0.20, 0.30));

    let max_loops = config.map(|c| c.acp.max_tool_loop_iterations).unwrap_or(25);

    let max_ctx = config.map(|c| c.acp.max_context_tokens).unwrap_or(220_000);

    let compress_threshold = config
        .map(|c| (c.acp.max_context_tokens as f64 * c.acp.compression_threshold as f64) as usize)
        .unwrap_or(165_000);

    // NOTE: Inside format!() the DOT brace characters must be doubled ({{ / }}).
    //       \\n is a literal backslash-n which DOT interprets as a line break inside labels.
    format!(
        "digraph GrokCLI {{\n\
         \x20 // -- Layout ----------------------------------------------------------\n\
         \x20 rankdir=TB;\n\
         \x20 splines=ortho;\n\
         \x20 nodesep=0.6;\n\
         \x20 ranksep=0.8;\n\
         \x20 fontname=\"Helvetica,Arial,sans-serif\";\n\
         \x20 node [fontname=\"Helvetica,Arial,sans-serif\", fontsize=11];\n\
         \x20 edge [fontname=\"Helvetica,Arial,sans-serif\", fontsize=10];\n\
         \n\
         \x20 // -- Entry -----------------------------------------------------------\n\
         \x20 UserInput [\n\
         \x20   label=\"User Input\\n(message / slash command)\",\n\
         \x20   shape=parallelogram,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#AED6F1\",\n\
         \x20   color=\"#2980B9\"\n\
         \x20 ];\n\
         \n\
         \x20 SlashDispatch [\n\
         \x20   label=\"Slash Command?\\n/clear /model /bayes\\n/goal /recall /archives\\n/visualize ...\",\n\
         \x20   shape=diamond,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#FAD7A0\",\n\
         \x20   color=\"#CA6F1E\"\n\
         \x20 ];\n\
         \n\
         \x20 BuiltinHandler [\n\
         \x20   label=\"Built-in Handler\\n(no AI round-trip)\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#A9DFBF\",\n\
         \x20   color=\"#1E8449\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Bayesian Router -------------------------------------------------\n\
         \x20 BayesRouter [\n\
         \x20   label=\"Bayesian Router\\n-----------------\\n\
                intent_edit     {p_edit:.2}\\n\
                intent_shell    {p_shell:.2}\\n\
                intent_search   {p_search:.2}\\n\
                intent_question {p_question:.2}\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#F9E79F\",\n\
         \x20   color=\"#B7950B\"\n\
         \x20 ];\n\
         \n\
         \x20 PromptRefiner [\n\
         \x20   label=\"Prompt Refiner\\n-----------------\\n\
                - Repetition detection\\n\
                - Uncertainty notes\\n\
                - Vagueness notes\\n\
                - Active goal injection\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#F9E79F\",\n\
         \x20   color=\"#B7950B\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Context Management ---------------------------------------------\n\
         \x20 ContextMgr [\n\
         \x20   label=\"Context Manager\\n-----------------\\n\
                1. Truncate tool results (30k chars)\\n\
                2. Count-based trim (80 msgs)\\n\
                3. Token-budget trim ({max_ctx} tokens)\\n\
                4. Smart compress (>{compress_threshold} tokens)\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#D2B4DE\",\n\
         \x20   color=\"#6C3483\"\n\
         \x20 ];\n\
         \n\
         \x20 ContextArchive [\n\
         \x20   label=\"Context Archive\\n~/.grok/sessions/{{id}}/archives/\\n\
                -----------------\\n\
                AI summarises oldest chunk\\n\
                Raw messages preserved\\n\
                /recall N to restore\",\n\
         \x20   shape=cylinder,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#D2B4DE\",\n\
         \x20   color=\"#6C3483\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- LLM API --------------------------------------------------------\n\
         \x20 GrokAPI [\n\
         \x20   label=\"Grok API\\n-----------------\\n\
                Starlink-safe: 5 retries\\n\
                5s/10s/20s/40s/60s backoff\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#FADBD8\",\n\
         \x20   color=\"#943126\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Tool Loop -------------------------------------------------------\n\
         \x20 ToolLoop [\n\
         \x20   label=\"Tool Loop\\n-----------------\\n\
                max {max_loops} iterations\\n\
                File / Shell / Web / Memory\\n\
                Task / Plan / Skill / MCP\\n\
                recall_context (context recall)\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#A9DFBF\",\n\
         \x20   color=\"#1E8449\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Memory Subsystem ------------------------------------------------\n\
         \x20 MemorySystem [\n\
         \x20   label=\"Memory Subsystem\\n-----------------\\n\
                Short-term  (session buffer)\\n\
                Long-term   (~/.grok/memory.json)\\n\
                Episodic    (~/.grok/sessions/)\\n\
                Working     (project context)\\n\
                Knowledge   (knowledge/)\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#AED6F1\",\n\
         \x20   color=\"#2980B9\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Session DNA -----------------------------------------------------\n\
         \x20 SessionDNA [\n\
         \x20   label=\"Session DNA\\nsession_dna.json\\n\
                -----------------\\n\
                tone / verbosity\\n\
                coding_style\\n\
                tool_preferences\",\n\
         \x20   shape=box,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#AED6F1\",\n\
         \x20   color=\"#2980B9\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Output ----------------------------------------------------------\n\
         \x20 Response [\n\
         \x20   label=\"Response\\n(streamed to client)\",\n\
         \x20   shape=parallelogram,\n\
         \x20   style=filled,\n\
         \x20   fillcolor=\"#A9DFBF\",\n\
         \x20   color=\"#1E8449\"\n\
         \x20 ];\n\
         \n\
         \x20 // -- Edges -----------------------------------------------------------\n\
         \x20 UserInput     -> SlashDispatch;\n\
         \x20 SlashDispatch -> BuiltinHandler [label=\"yes\"];\n\
         \x20 SlashDispatch -> PromptRefiner  [label=\"no\"];\n\
         \x20 BuiltinHandler -> Response;\n\
         \x20 PromptRefiner  -> ContextMgr;\n\
         \x20 ContextMgr     -> ContextArchive [label=\"tokens > {compress_threshold}\", style=dashed];\n\
         \x20 ContextArchive -> ContextMgr     [label=\"/recall N\", style=dashed];\n\
         \x20 ContextMgr     -> BayesRouter;\n\
         \x20 BayesRouter    -> GrokAPI;\n\
         \x20 GrokAPI        -> ToolLoop  [label=\"tool_calls\"];\n\
         \x20 GrokAPI        -> Response  [label=\"stop\"];\n\
         \x20 ToolLoop       -> GrokAPI   [label=\"tool results\"];\n\
         \x20 ToolLoop       -> MemorySystem [label=\"save_memory / recall_context\", style=dashed];\n\
         \x20 SessionDNA     -> PromptRefiner [label=\"inject at startup\", style=dashed];\n\
         \x20 MemorySystem   -> BayesRouter   [label=\"context at startup\", style=dashed];\n\
         }}\n",
        p_edit = p_edit,
        p_shell = p_shell,
        p_search = p_search,
        p_question = p_question,
        max_loops = max_loops,
        max_ctx = max_ctx,
        compress_threshold = compress_threshold,
    )
}

/// Return a Markdown-wrapped code block containing the DOT graph for
/// display in the ACP `/visualize` slash command response.
pub fn generate_pipeline_markdown(config: Option<&Config>) -> String {
    let dot = generate_pipeline_dot(config);
    format!(
        "## Grok-CLI Pipeline Graph (DOT/Graphviz)\n\n\
         Render with: `dot -Tsvg pipeline.dot -o pipeline.svg`\n\
         or: `grok visualize | dot -Tpng -o pipeline.png`\n\n\
         ```dot\n{dot}```"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dot_output_is_valid_digraph() {
        let dot = generate_pipeline_dot(None);
        assert!(dot.contains("digraph GrokCLI {"), "must start with digraph");
        assert!(dot.contains("UserInput"), "must include UserInput node");
        assert!(dot.contains("BayesRouter"), "must include BayesRouter");
        assert!(dot.contains("ToolLoop"), "must include ToolLoop");
        assert!(dot.contains("GrokAPI"), "must include GrokAPI");
        assert!(dot.contains("Response"), "must include Response");
    }

    #[test]
    fn dot_contains_default_priors() {
        let dot = generate_pipeline_dot(None);
        // Default intent_question prior is 0.30
        assert!(dot.contains("0.30"), "must show intent_question prior");
    }

    #[test]
    fn markdown_wraps_in_code_block() {
        let md = generate_pipeline_markdown(None);
        assert!(md.contains("```dot"), "must have dot code fence");
        assert!(md.contains("Render with"), "must have usage hint");
    }
}
