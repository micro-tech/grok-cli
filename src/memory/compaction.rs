//! Context Compaction Algorithm (Task 453)
//!
//! Deterministic, Rust-pure compaction for multi-slot memory.
//! Target: 50–80% token reduction while preserving meaning.
//!
//! This module is the implementation of the "Compaction Algorithm (Deterministic)"
//! section from docs/multi_slot_replace_memory_spec.md.

use crate::memory::replace_slot::SlotType;

/// Rough token estimator (consistent with the rest of the memory system).
pub fn estimate_tokens(text: &str) -> usize {
    if text.is_empty() {
        0
    } else {
        ((text.len() as f64) / 4.0).ceil() as usize
    }
}

/// Main entry point for compacting content to a target token budget.
///
/// This is the core deterministic compaction algorithm for Task 453.
pub fn compact_content(text: &str, slot_type: SlotType, target_tokens: usize) -> String {
    if text.trim().is_empty() {
        return String::new();
    }

    let original_tokens = estimate_tokens(text);
    if original_tokens <= target_tokens {
        return text.trim().to_string();
    }

    let mut working = text.to_string();

    // ── Step 1: Remove exact duplicate lines (keep first occurrence, preserve order)
    working = deduplicate_lines(&working);

    // ── Step 2: Slot-specific hard rules
    working = apply_slot_hard_rules(&working, &slot_type);

    // ── Step 3: Strip verbose logging, stack traces, and repetitive noise
    working = strip_verbose_sections(&working);

    // ── Step 4: Replace repeated long code blocks with references
    working = deduplicate_code_blocks(&working);

    // ── Step 5: Bulletize long dense paragraphs (improves readability + reduces tokens)
    working = bulletize_paragraphs(&working);

    // ── Step 6: Remove near-duplicate lines (fuzzy)
    working = remove_near_duplicates(&working);

    // ── Step 7: Final size reduction if still over budget
    let current = estimate_tokens(&working);
    if current > target_tokens {
        working = smart_truncate(&working, target_tokens, &slot_type);
    }

    // ── Step 8: Final cleanup
    let mut result = working.trim().to_string();

    // Never let plan become empty if it had content
    if slot_type == SlotType::Plan && result.is_empty() && !text.trim().is_empty() {
        result = text.trim().to_string();
    }

    // Ensure we didn't make it worse
    if estimate_tokens(&result) > original_tokens {
        result = text.trim().to_string(); // safety fallback
    }

    result
}

/// Remove exact duplicate lines while preserving order of first appearance.
fn deduplicate_lines(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for line in text.lines() {
        let key = line.trim();
        if key.is_empty() {
            out.push(line.to_string());
            continue;
        }
        if seen.insert(key.to_string()) {
            out.push(line.to_string());
        }
    }

    out.join("\n")
}

/// Remove near-duplicate lines using simple normalization.
fn remove_near_duplicates(text: &str) -> String {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();

    for line in text.lines() {
        let normalized = normalize_for_dedup(line);
        if normalized.is_empty() {
            out.push(line.to_string());
            continue;
        }
        if seen.insert(normalized) {
            out.push(line.to_string());
        }
    }

    out.join("\n")
}

fn normalize_for_dedup(line: &str) -> String {
    line.to_lowercase()
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Apply hard rules per slot type.
fn apply_slot_hard_rules(text: &str, slot_type: &SlotType) -> String {
    match slot_type {
        SlotType::Errors => {
            // Always keep the most recent 3 error blocks
            keep_most_recent_errors(text, 3)
        }
        SlotType::Plan => {
            // Plan is almost sacred — only do light dedup
            text.to_string()
        }
        _ => text.to_string(),
    }
}

/// Keep only the most recent N error entries.
/// Errors are often separated by blank lines or "error:" markers.
fn keep_most_recent_errors(text: &str, keep_last: usize) -> String {
    let blocks: Vec<&str> = text
        .split("\n\n")
        .filter(|b| !b.trim().is_empty())
        .collect();

    if blocks.len() <= keep_last {
        return text.to_string();
    }

    let kept: Vec<&str> = blocks
        .into_iter()
        .rev()
        .take(keep_last)
        .rev()
        .collect();

    kept.join("\n\n")
}

/// Strip verbose sections: long stack traces, repeated logs, debug dumps.
fn strip_verbose_sections(text: &str) -> String {
    let mut result = String::new();
    let mut in_stack = false;
    let mut stack_lines = 0;

    for line in text.lines() {
        let l = line.trim();

        // Detect start of stack trace
        if l.starts_with("thread '") || l.contains("panicked at") || l.starts_with("stack backtrace:") {
            in_stack = true;
            stack_lines = 0;
            result.push_str("[stack trace summarized]\n");
            continue;
        }

        if in_stack {
            stack_lines += 1;
            if stack_lines > 2 {
                // skip deep stack frames
                continue;
            }
            if l.starts_with("   ") || l.contains("::") {
                continue;
            }
            if l.is_empty() || l.starts_with("note:") {
                in_stack = false;
            }
        }

        // Skip extremely repetitive log lines
        if l.starts_with("DEBUG ") || l.starts_with("TRACE ") {
            continue;
        }

        result.push_str(line);
        result.push('\n');
    }

    result
}

/// Replace repeated long code blocks with a reference.
fn deduplicate_code_blocks(text: &str) -> String {
    // Simple heuristic: if we see the same block of 8+ lines twice, replace second occurrence
    let lines: Vec<&str> = text.lines().collect();
    let mut seen_blocks = std::collections::HashMap::new();
    let mut output: Vec<String> = Vec::new();

    let mut i = 0;
    while i < lines.len() {
        // Look for potential code blocks (indented or fenced)
        let is_code_start = lines[i].starts_with("```") || lines[i].starts_with("    ");

        if is_code_start {
            let mut block_end = i + 1;
            while block_end < lines.len() && block_end < i + 30 {
                if lines[block_end].starts_with("```") && block_end > i + 3 {
                    block_end += 1;
                    break;
                }
                block_end += 1;
            }

            let block = lines[i..block_end].join("\n");
            let key = normalize_for_dedup(&block);

            if block.len() > 80 && seen_blocks.contains_key(&key) {
                output.push(format!("[see repeated code block from earlier in {}]", seen_blocks[&key]));
                i = block_end;
                continue;
            } else if block.len() > 80 {
                seen_blocks.insert(key, format!("mem.X")); // placeholder; real manager knows context
            }

            // Convert the slice of &str to owned Strings
            for line in &lines[i..block_end] {
                output.push(line.to_string());
            }
            i = block_end;
        } else {
            output.push(lines[i].to_string());
            i += 1;
        }
    }

    output.join("\n")
}

/// Turn long dense paragraphs into bullet points.
fn bulletize_paragraphs(text: &str) -> String {
    let mut result = String::new();

    for para in text.split("\n\n") {
        let trimmed = para.trim();
        if trimmed.len() < 120 {
            result.push_str(trimmed);
            result.push_str("\n\n");
            continue;
        }

        // Only bulletize if it looks like prose (not code)
        if trimmed.contains("```") || trimmed.lines().any(|l| l.starts_with("    ") || l.starts_with("\t")) {
            result.push_str(trimmed);
            result.push_str("\n\n");
            continue;
        }

        // Split sentences roughly
        let sentences: Vec<&str> = trimmed
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| s.trim().len() > 8)
            .collect();

        if sentences.len() >= 3 {
            for s in sentences {
                let s = s.trim();
                if !s.is_empty() {
                    result.push_str(&format!("- {}\n", s));
                }
            }
            result.push('\n');
        } else {
            result.push_str(trimmed);
            result.push_str("\n\n");
        }
    }

    result.trim().to_string()
}

/// Final truncation that tries to preserve the most important parts.
fn smart_truncate(text: &str, target_tokens: usize, slot_type: &SlotType) -> String {
    let target_chars = target_tokens * 4;

    if text.len() <= target_chars {
        return text.to_string();
    }

    match slot_type {
        SlotType::Plan => {
            // Be very conservative with plan
            let keep = target_chars.min(text.len());
            text.chars().take(keep).collect()
        }
        SlotType::Errors => {
            // Keep the end (most recent)
            let start = text.len().saturating_sub(target_chars);
            format!("[older errors truncated]\n{}", &text[start..])
        }
        _ => {
            // For working/context/mem: keep head + tail
            let head_chars = (target_chars as f64 * 0.55) as usize;
            let tail_chars = (target_chars as f64 * 0.35) as usize;

            let head: String = text.chars().take(head_chars).collect();
            let tail: String = text
                .chars()
                .rev()
                .take(tail_chars)
                .collect::<String>()
                .chars()
                .rev()
                .collect();

            format!(
                "{}\n\n… [compacted {} tokens] …\n\n{}",
                head.trim_end(),
                estimate_tokens(text) - target_tokens,
                tail.trim_start()
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dedup_removes_exact_duplicates() {
        let input = "hello\nworld\nhello\nfoo\nworld";
        let out = deduplicate_lines(input);
        assert_eq!(out, "hello\nworld\nfoo");
    }

    #[test]
    fn errors_slot_keeps_recent() {
        let input = "err1\n\nerr2\n\nerr3\n\nerr4\n\nerr5";
        let out = keep_most_recent_errors(input, 3);
        assert!(out.contains("err3"));
        assert!(out.contains("err4"));
        assert!(out.contains("err5"));
        assert!(!out.contains("err1"));
    }

    #[test]
    fn compaction_reduces_size() {
        let long_text = "This is a very long paragraph about something important. ".repeat(20);
        let compacted = compact_content(&long_text, SlotType::Context, 50);
        assert!(estimate_tokens(&compacted) <= 60);
        assert!(compacted.len() < long_text.len());
    }

    #[test]
    fn plan_is_protected() {
        let plan = "Goal 1: do X\nGoal 2: do Y\n".repeat(30);
        let compacted = compact_content(&plan, SlotType::Plan, 30);
        // Should not destroy the plan
        assert!(compacted.contains("Goal 1"));
    }

    #[test]
    fn verbose_stack_is_stripped() {
        let input = "Something failed\nthread 'main' panicked at 'boom', src/main.rs:10\n   0: core::panicking\n   1: my_crate::foo\n   2: bar\nBacktrace done\nNormal line after";
        let out = strip_verbose_sections(input);
        assert!(out.contains("[stack trace summarized]"));
        assert!(out.contains("Normal line after"));
        assert!(!out.contains("core::panicking"));
    }
}