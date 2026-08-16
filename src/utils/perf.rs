//! Simple performance measurement helpers for Task 266.
//!
//! Usage:
//!   GROK_PERF=1 cargo run -- chat "hello"
//!
//! This will print lines like:
//!   [perf] interactive turn took 1.23s

use std::time::Instant;

/// Returns true if performance timing output is enabled.
#[inline]
pub fn perf_enabled() -> bool {
    std::env::var_os("GROK_PERF").is_some()
}

/// Start a turn timer (cheap).
#[inline]
pub fn start_turn() -> Instant {
    Instant::now()
}

/// Report elapsed time for a labeled turn if `GROK_PERF` is set.
#[inline]
pub fn report_turn(label: &str, start: Instant) {
    if perf_enabled() {
        eprintln!("[perf] {} took {:.2?}", label, start.elapsed());
    }
}

/// Convenience macro for the common pattern:
/// ```ignore
/// let _t = perf_guard!("my label");
/// // ... work ...
/// ```
#[macro_export]
macro_rules! perf_guard {
    ($label:expr) => {
        let __start = if $crate::utils::perf::perf_enabled() {
            Some($crate::utils::perf::start_turn())
        } else {
            None
        };
        // On drop we report (only if enabled)
        let _guard = $crate::utils::perf::PerfGuard {
            label: $label,
            start: __start,
        };
    };
}

/// RAII guard that reports on drop when perf is enabled.
pub struct PerfGuard {
    pub label: &'static str,
    pub start: Option<Instant>,
}

impl Drop for PerfGuard {
    fn drop(&mut self) {
        if let Some(start) = self.start {
            eprintln!("[perf] {} took {:.2?}", self.label, start.elapsed());
        }
    }
}

/// Simple built-in "benchmark" helper for Task 266.
/// Runs `f` and reports time under the given label when GROK_PERF=1.
/// Returns the result of `f`.
///
/// Example (in a test or CLI):
/// ```ignore
/// let result = time_it("my turn", || { do_work() });
/// ```
pub fn time_it<T, F: FnOnce() -> T>(label: &str, f: F) -> T {
    let start = start_turn();
    let result = f();
    report_turn(label, start);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn perf_smoke_test() {
        // This test always runs timing (for harness purposes).
        // Real use is gated by GROK_PERF.
        let _ = time_it("perf_smoke", || {
            // Simulate a cheap "turn"
            std::thread::sleep(std::time::Duration::from_millis(5));
            42
        });
        // If GROK_PERF=1 this will have printed.
        // The test itself always passes.
    }
}
