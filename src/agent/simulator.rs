//! Simulation Mode (Dry-Run Engine)
//!
//! Provides a dry-run engine that predicts tool calls and outcomes without
//! executing anything. Used by the `/simulate` command in interactive mode.

use std::collections::HashSet;

// ─── Risk Classification ─────────────────────────────────────────────────────

/// Risk level assigned to a simulated operation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RiskLevel {
    /// Read-only operations – completely safe.
    Safe,
    /// Writes or creates files – recoverable but modifies state.
    Caution,
    /// Shell commands, deletes, or overwrites – hard to undo.
    Destructive,
    /// Could not be classified.
    Unknown,
}

impl RiskLevel {
    /// Derive a risk level purely from a known tool name.
    pub fn from_tool_name(tool_name: &str) -> Self {
        match tool_name.to_lowercase().as_str() {
            "read_file" | "list_directory" | "glob_search" | "web_search" | "web_fetch" => {
                RiskLevel::Safe
            }
            "write_file" | "replace" | "create_directory" => RiskLevel::Caution,
            "run_shell_command" | "delete_file" | "delete_directory" => RiskLevel::Destructive,
            "text_reply" => RiskLevel::Safe,
            _ => RiskLevel::Unknown,
        }
    }

    /// Short uppercase label for display.
    pub fn label(&self) -> &'static str {
        match self {
            RiskLevel::Safe => "SAFE",
            RiskLevel::Caution => "CAUTION",
            RiskLevel::Destructive => "DESTRUCTIVE",
            RiskLevel::Unknown => "UNKNOWN",
        }
    }

    /// Colour-coded label string (uses ANSI via the `colored` crate).
    pub fn coloured_label(&self) -> String {
        use colored::*;
        match self {
            RiskLevel::Safe => "SAFE".bright_green().to_string(),
            RiskLevel::Caution => "CAUTION".bright_yellow().to_string(),
            RiskLevel::Destructive => "DESTRUCTIVE".bright_red().to_string(),
            RiskLevel::Unknown => "UNKNOWN".dimmed().to_string(),
        }
    }

    /// Merge two risk levels, keeping the higher one.
    pub fn escalate(self, other: &RiskLevel) -> Self {
        match (&self, other) {
            (RiskLevel::Destructive, _) | (_, RiskLevel::Destructive) => RiskLevel::Destructive,
            (RiskLevel::Caution, _) | (_, RiskLevel::Caution) => RiskLevel::Caution,
            (RiskLevel::Unknown, _) | (_, RiskLevel::Unknown) => RiskLevel::Unknown,
            _ => RiskLevel::Safe,
        }
    }
}

// ─── Data Types ──────────────────────────────────────────────────────────────

/// One predicted operation within a simulation.
#[derive(Debug, Clone)]
pub struct SimulatedOperation {
    pub step: usize,
    pub tool_name: String,
    pub description: String,
    pub risk_level: RiskLevel,
    /// Key/value argument pairs extracted from the model's plan.
    pub arguments: Vec<(String, String)>,
    /// Per-operation warnings produced by local safety analysis.
    pub warnings: Vec<String>,
}

/// The full result of a simulation run.
#[derive(Debug)]
pub struct SimulationResult {
    pub operations: Vec<SimulatedOperation>,
    /// Contradiction messages detected by local analysis.
    pub contradictions: Vec<String>,
    /// Highest risk level across all operations.
    pub overall_risk: RiskLevel,
    /// One-sentence summary extracted from the model's response.
    pub summary: String,
}

impl SimulationResult {
    /// Returns `true` if there are any warnings or destructive operations.
    pub fn has_warnings(&self) -> bool {
        !self.contradictions.is_empty()
            || self
                .operations
                .iter()
                .any(|op| !op.warnings.is_empty() || op.risk_level == RiskLevel::Destructive)
    }
}

// ─── Simulation System Prompt ────────────────────────────────────────────────

/// System-prompt fragment injected when running in simulation mode.
///
/// The model is instructed to describe its plan in a parseable structured
/// format rather than actually invoking any tools.
pub const SIMULATION_SYSTEM_PROMPT: &str = "\
## SIMULATION MODE (DRY-RUN) — ACTIVE

You are operating in SIMULATION MODE. You MUST NOT invoke any tools or perform \
any real actions. Instead, analyse the user's request and respond with a \
structured plan that describes exactly what you WOULD do.

Use this EXACT format (do not deviate):

---SIMULATION START---
STEP 1: <tool_name>
  Description: <what this step does>
  Risk: <SAFE | CAUTION | DESTRUCTIVE>
  Key args: <key=value, key=value, …>

STEP 2: <tool_name>
  Description: …
  Risk: …
  Key args: …
---SIMULATION END---
SUMMARY: <one sentence describing the expected overall outcome>

Risk guide:
  SAFE        — read_file, list_directory, glob_search, web_search, web_fetch
  CAUTION     — write_file, replace, create_directory  (modifies files, recoverable)
  DESTRUCTIVE — run_shell_command, delete_file, delete_directory  (hard to undo)

If you would NOT call any tools and would reply with plain text only, write:
---SIMULATION START---
STEP 1: text_reply
  Description: Respond with a plain-text explanation.
  Risk: SAFE
  Key args: none
---SIMULATION END---
SUMMARY: <what your text response would cover>

Do NOT include any content outside the markers above (no prose before or after).";

// ─── Parser ──────────────────────────────────────────────────────────────────

/// Parse the structured simulation response produced by the model.
///
/// The parser is intentionally lenient: it handles extra whitespace, missing
/// fields, and partial responses gracefully.
pub fn parse_simulation_response(response: &str) -> SimulationResult {
    let mut operations: Vec<SimulatedOperation> = Vec::new();
    let mut summary = String::from("No summary provided.");

    // State machine
    let mut in_block = false;
    let mut step_index: Option<usize> = None;
    let mut tool_name = String::new();
    let mut description = String::new();
    let mut risk_override: Option<RiskLevel> = None;
    let mut arguments: Vec<(String, String)> = Vec::new();

    let flush = |ops: &mut Vec<SimulatedOperation>,
                 idx: Option<usize>,
                 tn: &mut String,
                 desc: &mut String,
                 risk_ov: &mut Option<RiskLevel>,
                 args: &mut Vec<(String, String)>| {
        if let Some(step) = idx {
            if !tn.is_empty() {
                let risk = risk_ov
                    .take()
                    .unwrap_or_else(|| RiskLevel::from_tool_name(tn));
                let mut warnings = Vec::new();
                build_warnings(tn, &risk, &mut warnings);
                ops.push(SimulatedOperation {
                    step,
                    tool_name: std::mem::take(tn),
                    description: std::mem::take(desc),
                    risk_level: risk,
                    arguments: std::mem::take(args),
                    warnings,
                });
            }
        }
    };

    for raw in response.lines() {
        let line = raw.trim();

        // ── block delimiters ────────────────────────────────────────────────
        if line == "---SIMULATION START---" {
            in_block = true;
            continue;
        }
        if line == "---SIMULATION END---" {
            flush(
                &mut operations,
                step_index.take(),
                &mut tool_name,
                &mut description,
                &mut risk_override,
                &mut arguments,
            );
            in_block = false;
            continue;
        }

        // ── summary line (outside or inside block) ──────────────────────────
        if let Some(rest) = line.strip_prefix("SUMMARY:") {
            summary = rest.trim().to_string();
            continue;
        }

        if !in_block {
            continue;
        }

        // ── STEP N: tool_name ───────────────────────────────────────────────
        if let Some(rest) = line.strip_prefix("STEP ") {
            // flush previous step
            flush(
                &mut operations,
                step_index.take(),
                &mut tool_name,
                &mut description,
                &mut risk_override,
                &mut arguments,
            );
            description.clear();
            risk_override = None;
            arguments.clear();

            if let Some(colon) = rest.find(':') {
                let num_str = rest[..colon].trim();
                let tn = rest[colon + 1..].trim().to_string();
                step_index = num_str.parse::<usize>().ok().or(Some(operations.len() + 1));
                tool_name = tn;
            }
            continue;
        }

        // ── field lines ─────────────────────────────────────────────────────
        if let Some(rest) = line.strip_prefix("Description:") {
            description = rest.trim().to_string();
            continue;
        }

        if let Some(rest) = line.strip_prefix("Risk:") {
            risk_override = Some(match rest.trim().to_uppercase().as_str() {
                "SAFE" => RiskLevel::Safe,
                "CAUTION" => RiskLevel::Caution,
                "DESTRUCTIVE" => RiskLevel::Destructive,
                _ => RiskLevel::from_tool_name(&tool_name),
            });
            continue;
        }

        if let Some(rest) = line.strip_prefix("Key args:") {
            for pair in rest.split(',') {
                let pair = pair.trim();
                if pair.eq_ignore_ascii_case("none") || pair.is_empty() {
                    continue;
                }
                if let Some(eq) = pair.find('=') {
                    arguments.push((
                        pair[..eq].trim().to_string(),
                        pair[eq + 1..].trim().to_string(),
                    ));
                } else {
                    arguments.push((pair.to_string(), String::new()));
                }
            }
        }
    }

    // Flush any remaining step if END marker was missing
    if in_block {
        flush(
            &mut operations,
            step_index.take(),
            &mut tool_name,
            &mut description,
            &mut risk_override,
            &mut arguments,
        );
    }

    // ── contradiction analysis ───────────────────────────────────────────────
    let contradictions = detect_contradictions(&operations);

    // ── overall risk ─────────────────────────────────────────────────────────
    let overall_risk = operations
        .iter()
        .fold(RiskLevel::Safe, |acc, op| acc.escalate(&op.risk_level));

    SimulationResult {
        operations,
        contradictions,
        overall_risk,
        summary,
    }
}

/// Populate per-operation warnings based on tool name and risk level.
fn build_warnings(tool_name: &str, risk: &RiskLevel, warnings: &mut Vec<String>) {
    match tool_name {
        "run_shell_command" => {
            warnings.push(
                "Shell commands can have unpredictable side-effects on the system.".to_string(),
            );
        }
        "delete_file" | "delete_directory" => {
            warnings.push(format!(
                "`{}` permanently removes data — ensure you have a backup.",
                tool_name
            ));
        }
        "write_file" => {
            warnings.push(
                "Writing a file will overwrite any existing content at that path.".to_string(),
            );
        }
        _ => {}
    }
    if *risk == RiskLevel::Unknown {
        warnings.push(format!(
            "Tool `{}` is not recognised — risk level cannot be determined.",
            tool_name
        ));
    }
}

// ─── Contradiction Detector ───────────────────────────────────────────────────

/// Detect logical conflicts or ordering issues between simulated operations.
fn detect_contradictions(ops: &[SimulatedOperation]) -> Vec<String> {
    let mut issues: Vec<String> = Vec::new();

    // Track files that have been written so far
    let mut written: HashSet<String> = HashSet::new();
    // Track the last write step index for ordering checks
    let mut last_write_step: Option<usize> = None;

    for op in ops {
        // Extract the primary path argument (if any)
        let path_arg = op
            .arguments
            .iter()
            .find(|(k, _)| matches!(k.as_str(), "path" | "file" | "filename" | "src" | "dest"))
            .map(|(_, v)| v.as_str());

        match op.tool_name.as_str() {
            "read_file" => {
                if let Some(path) = path_arg {
                    if written.contains(path) {
                        issues.push(format!(
                            "Step {}: reading '{}' after it has already been written — \
                             ensure the write completes successfully first.",
                            op.step, path
                        ));
                    }
                }
            }
            "write_file" | "replace" => {
                if let Some(path) = path_arg {
                    if written.contains(path) {
                        issues.push(format!(
                            "Step {}: '{}' is written more than once — \
                             the later write will overwrite the earlier one.",
                            op.step, path
                        ));
                    }
                    written.insert(path.to_string());
                } else {
                    written.insert(format!("<unknown path step {}>", op.step));
                }
                last_write_step = Some(op.step);
            }
            "run_shell_command" => {
                if let Some(write_step) = last_write_step {
                    issues.push(format!(
                        "Step {}: shell command runs after a file write at step {} — \
                         verify the file is fully flushed before the command reads it.",
                        op.step, write_step
                    ));
                }
            }
            "delete_file" | "delete_directory" => {
                if let Some(path) = path_arg {
                    if written.contains(path) {
                        issues.push(format!(
                            "Step {}: '{}' is written then deleted in the same plan — \
                             this looks like a no-op or an error.",
                            op.step, path
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    // Check for duplicate destructive shell commands (same arguments)
    let shell_cmds: Vec<String> = ops
        .iter()
        .filter(|op| op.tool_name == "run_shell_command")
        .flat_map(|op| op.arguments.iter().map(|(_, v)| v.clone()))
        .collect();
    let mut seen_cmds: HashSet<&String> = HashSet::new();
    for cmd in &shell_cmds {
        if !cmd.is_empty() && !seen_cmds.insert(cmd) {
            issues.push(format!(
                "Shell command `{}` appears more than once — possible duplication.",
                cmd
            ));
        }
    }

    issues
}

// ─── Display ──────────────────────────────────────────────────────────────────

/// Print a formatted simulation report to stdout.
pub fn display_simulation_result(result: &SimulationResult, original_input: &str) {
    use colored::*;

    println!();
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue()
    );
    println!(
        "{}  {}",
        "🔬 SIMULATION RESULT".bright_blue().bold(),
        format!("(dry-run — nothing was executed)").dimmed()
    );
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue()
    );

    // Input echo
    let truncated = if original_input.len() > 80 {
        format!("{}…", &original_input[..80])
    } else {
        original_input.to_string()
    };
    println!("  {} {}", "Input:".dimmed(), truncated.bright_white());
    println!();

    // Operations
    if result.operations.is_empty() {
        println!("  {} No operations predicted.", "ℹ".bright_blue());
    } else {
        println!(
            "  {} {} operation(s) predicted:",
            "📋".to_string(),
            result.operations.len().to_string().bright_white()
        );
        println!();

        for op in &result.operations {
            let risk_badge = op.risk_level.coloured_label();
            println!(
                "  {} {}  [{}]",
                format!("Step {}:", op.step).bright_white().bold(),
                op.tool_name.bright_cyan(),
                risk_badge
            );

            if !op.description.is_empty() {
                println!("       {}", op.description.dimmed());
            }

            if !op.arguments.is_empty() {
                let args_str = op
                    .arguments
                    .iter()
                    .map(|(k, v)| {
                        if v.is_empty() {
                            k.clone()
                        } else {
                            format!("{}={}", k.bright_white(), v.dimmed())
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                println!("       {} {}", "args:".dimmed(), args_str);
            }

            for warn in &op.warnings {
                println!("       {} {}", "⚠".bright_yellow(), warn.bright_yellow());
            }

            println!();
        }
    }

    // Contradictions
    if !result.contradictions.is_empty() {
        println!(
            "  {} {} contradiction(s) detected:",
            "⚡".bright_yellow(),
            result
                .contradictions
                .len()
                .to_string()
                .bright_yellow()
                .bold()
        );
        for c in &result.contradictions {
            println!("    {} {}", "→".bright_yellow(), c.yellow());
        }
        println!();
    }

    // Summary
    println!("  {} {}", "Summary:".bright_white().bold(), result.summary);
    println!();

    // Overall risk banner
    let risk_line = match result.overall_risk {
        RiskLevel::Safe => format!(
            "  {} Overall risk: {}",
            "✓".bright_green(),
            "SAFE — safe to proceed".bright_green()
        ),
        RiskLevel::Caution => format!(
            "  {} Overall risk: {}",
            "⚠".bright_yellow(),
            "CAUTION — review file changes before proceeding".bright_yellow()
        ),
        RiskLevel::Destructive => format!(
            "  {} Overall risk: {}",
            "✗".bright_red(),
            "DESTRUCTIVE — dangerous operations detected, proceed with care".bright_red()
        ),
        RiskLevel::Unknown => format!(
            "  {} Overall risk: {}",
            "?".dimmed(),
            "UNKNOWN — could not fully assess risk".dimmed()
        ),
    };
    println!("{}", risk_line);
    println!(
        "{}",
        "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".bright_blue()
    );
    println!();
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn make_response(body: &str) -> String {
        format!(
            "---SIMULATION START---\n{}\n---SIMULATION END---\nSUMMARY: Test summary.",
            body
        )
    }

    #[test]
    fn test_risk_level_from_tool_name() {
        assert_eq!(RiskLevel::from_tool_name("read_file"), RiskLevel::Safe);
        assert_eq!(RiskLevel::from_tool_name("web_search"), RiskLevel::Safe);
        assert_eq!(RiskLevel::from_tool_name("write_file"), RiskLevel::Caution);
        assert_eq!(RiskLevel::from_tool_name("replace"), RiskLevel::Caution);
        assert_eq!(
            RiskLevel::from_tool_name("run_shell_command"),
            RiskLevel::Destructive
        );
        assert_eq!(
            RiskLevel::from_tool_name("delete_file"),
            RiskLevel::Destructive
        );
        assert_eq!(
            RiskLevel::from_tool_name("mystery_tool"),
            RiskLevel::Unknown
        );
    }

    #[test]
    fn test_risk_level_escalate() {
        assert_eq!(
            RiskLevel::Safe.escalate(&RiskLevel::Caution),
            RiskLevel::Caution
        );
        assert_eq!(
            RiskLevel::Caution.escalate(&RiskLevel::Destructive),
            RiskLevel::Destructive
        );
        assert_eq!(
            RiskLevel::Destructive.escalate(&RiskLevel::Safe),
            RiskLevel::Destructive
        );
        assert_eq!(RiskLevel::Safe.escalate(&RiskLevel::Safe), RiskLevel::Safe);
    }

    #[test]
    fn test_parse_single_safe_step() {
        let response = make_response(
            "STEP 1: read_file\n  Description: Read the main source file.\n  Risk: SAFE\n  Key args: path=src/main.rs",
        );
        let result = parse_simulation_response(&response);
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.operations[0].tool_name, "read_file");
        assert_eq!(result.operations[0].risk_level, RiskLevel::Safe);
        assert_eq!(result.operations[0].step, 1);
        assert_eq!(result.summary, "Test summary.");
    }

    #[test]
    fn test_parse_multiple_steps() {
        let body = "\
STEP 1: read_file
  Description: Read config.
  Risk: SAFE
  Key args: path=config.toml
STEP 2: write_file
  Description: Write updated config.
  Risk: CAUTION
  Key args: path=config.toml, content=…
STEP 3: run_shell_command
  Description: Restart the service.
  Risk: DESTRUCTIVE
  Key args: command=systemctl restart app";
        let response = make_response(body);
        let result = parse_simulation_response(&response);
        assert_eq!(result.operations.len(), 3);
        assert_eq!(result.overall_risk, RiskLevel::Destructive);
    }

    #[test]
    fn test_parse_text_reply_only() {
        let body = "\
STEP 1: text_reply
  Description: Respond with a plain-text explanation.
  Risk: SAFE
  Key args: none";
        let response = make_response(body);
        let result = parse_simulation_response(&response);
        assert_eq!(result.operations.len(), 1);
        assert_eq!(result.operations[0].tool_name, "text_reply");
        assert_eq!(result.overall_risk, RiskLevel::Safe);
        assert!(!result.has_warnings());
    }

    #[test]
    fn test_contradiction_double_write() {
        let body = "\
STEP 1: write_file
  Description: Write initial content.
  Risk: CAUTION
  Key args: path=output.txt
STEP 2: write_file
  Description: Overwrite with new content.
  Risk: CAUTION
  Key args: path=output.txt";
        let response = make_response(body);
        let result = parse_simulation_response(&response);
        assert!(
            result
                .contradictions
                .iter()
                .any(|c| c.contains("output.txt")),
            "Expected a double-write contradiction for output.txt"
        );
    }

    #[test]
    fn test_contradiction_write_then_shell() {
        let body = "\
STEP 1: write_file
  Description: Write script.
  Risk: CAUTION
  Key args: path=run.sh
STEP 2: run_shell_command
  Description: Execute script.
  Risk: DESTRUCTIVE
  Key args: command=bash run.sh";
        let response = make_response(body);
        let result = parse_simulation_response(&response);
        assert!(
            result
                .contradictions
                .iter()
                .any(|c| c.contains("shell command")),
            "Expected a shell-after-write contradiction"
        );
    }

    #[test]
    fn test_no_operations_returns_safe() {
        let result = parse_simulation_response("Nothing useful here.");
        assert!(result.operations.is_empty());
        assert_eq!(result.overall_risk, RiskLevel::Safe);
    }

    #[test]
    fn test_has_warnings_destructive() {
        let body = "\
STEP 1: delete_file
  Description: Remove temp file.
  Risk: DESTRUCTIVE
  Key args: path=tmp/cache.bin";
        let response = make_response(body);
        let result = parse_simulation_response(&response);
        assert!(result.has_warnings());
    }

    #[test]
    fn test_summary_extracted() {
        let response = "\
---SIMULATION START---
STEP 1: read_file
  Description: Read file.
  Risk: SAFE
  Key args: path=foo.txt
---SIMULATION END---
SUMMARY: The file will be read and its contents displayed.";
        let result = parse_simulation_response(response);
        assert_eq!(
            result.summary,
            "The file will be read and its contents displayed."
        );
    }
}
