//! Tests for Safety Hooks

#[cfg(test)]
mod tests {
    use super::*;
    use crate::safety::*;

    #[test]
    fn pre_write_blocks_binary_junk() {
        let ctx = WriteContext {
            path: std::path::Path::new("test.bin"),
            operation: "write",
            proposed_content: Some("\0\0\0\0binary junk"),
            diff: None,
            session_dna: None,
        };
        let decision = on_before_write_file(&ctx);
        assert!(matches!(decision, SafetyDecision::Block(_)));
    }

    #[test]
    fn diff_validator_rejects_large_full_rewrite() {
        let result = DiffValidator::validate_edit(50, 300, true);
        assert!(result.is_err());
    }

    #[test]
    fn intent_validator_rejects_ambiguous_request() {
        let result = IntentValidator::validate_intent("fix the bug", None);
        assert!(result.is_err());
    }

    #[test]
    fn suspicious_guard_rejects_empty_overwrite() {
        let result = SuspiciousWriteGuard::check(100, 0, "", None);
        assert!(result.is_err());
    }

    #[test]
    fn dna_safety_enters_safe_mode_on_failures() {
        let dna = serde_json::json!({
            "repeated_file_write_failures": 5
        });
        assert!(DnaSafetyController::should_enter_safe_mode(&dna));
    }

    #[test]
    fn tool_health_monitor_tracks_failures() {
        let monitor = ToolHealthMonitor::new();
        // is_healthy only evaluates failure rate once total >= 5
        monitor.record_failure("write_file");
        monitor.record_failure("write_file");
        monitor.record_failure("write_file");
        monitor.record_failure("write_file");
        monitor.record_failure("write_file");
        // 5 failures, 0 successes → failure rate = 1.0 > 0.35 → unhealthy
        assert!(!monitor.is_healthy("write_file"));
    }
}
