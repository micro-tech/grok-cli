//! Performance benchmarks for Task 266 (subtask 266.2).
//!
//! Run with:
//!   cargo bench
//!
//! These micro-benchmarks target the hottest per-turn paths identified during
//! Tasks 263-267 (tool defs, message construction, Bayesian routing, etc.).
//! They are intentionally small and deterministic so they can be run quickly
//! and compared before/after optimizations.

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use std::time::Duration;

use grok_cli::utils::perf;
use grok_cli::bayes::BayesianEngine;
use grok_cli::agent::router::Router;
use grok_cli::utils::messages;

// ---------------------------------------------------------------------------
// 1. Perf harness itself (baseline — should be extremely cheap when disabled)
// ---------------------------------------------------------------------------
fn bench_perf_helpers(c: &mut Criterion) {
    c.bench_function("perf_start_and_report_disabled", |b| {
        b.iter(|| {
            let start = perf::start_turn();
            let _ = black_box(42u32);
            // report_turn is a no-op when GROK_PERF is not set
            perf::report_turn("bench", start);
            black_box(start)
        })
    });
}

// ---------------------------------------------------------------------------
// 2. Tool registry (Task 263 win) — must be near-zero after OnceLock cache
// ---------------------------------------------------------------------------
fn bench_tool_registry(c: &mut Criterion) {
    c.bench_function("get_full_tool_definitions", |b| {
        b.iter(|| {
            let defs = grok_cli::tools::registry::get_full_tool_definitions();
            black_box(defs.len())
        })
    });

    c.bench_function("get_required_parameters_read_file", |b| {
        b.iter(|| {
            let req = grok_cli::tools::registry::get_required_parameters("read_file");
            black_box(req)
        })
    });
}

// ---------------------------------------------------------------------------
// 3. Cheap message construction (Task 267 direction)
// ---------------------------------------------------------------------------
fn bench_message_construction(c: &mut Criterion) {
    c.bench_function("messages_user", |b| {
        b.iter(|| {
            let msg = messages::user(black_box("hello world this is a test message"));
            black_box(msg)
        })
    });

    c.bench_function("messages_assistant_with_tool_calls", |b| {
        let tool_calls = vec![serde_json::json!({"id": "call_1", "type": "function"})];
        b.iter(|| {
            let msg = messages::assistant_with_tool_calls(
                Some(black_box("Here is the result".to_string())),
                tool_calls.clone(),
            );
            black_box(msg)
        })
    });
}

// ---------------------------------------------------------------------------
// 4. Bayesian engine + Router (core of per-turn intent routing)
// ---------------------------------------------------------------------------
fn bench_bayesian_and_router(c: &mut Criterion) {
    c.bench_function("bayesian_update_from_text", |b| {
        let mut engine = BayesianEngine::new_with_default_priors();
        b.iter(|| {
            engine.update_from_text(black_box("please edit the config file and run tests"));
            black_box(engine.best_intent())
        })
    });

    c.bench_function("router_route_simple", |b| {
        // Use default_priors variant for deterministic cheap benchmark
        let mut router = Router::new_with_default_priors();
        b.iter(|| {
            // We can't easily await in criterion sync bench, so we just exercise
            // the update path that route() calls. Full async route is harder.
            // This still exercises the hot Bayesian + intent logic.
            let _ = black_box(router.route_sync_for_bench(black_box("run a shell command")));
        })
    });
}

// ---------------------------------------------------------------------------
// 5. Small combined "typical light turn" micro-path
//    (message build + bayes + tool lookup)
// ---------------------------------------------------------------------------
fn bench_light_turn_micro(c: &mut Criterion) {
    c.bench_function("light_turn_micro", |b| {
        b.iter(|| {
            // Simulate what happens on a very light non-tool turn
            let _user = messages::user(black_box("what is the weather"));
            let mut engine = BayesianEngine::new_with_default_priors();
            engine.update_from_text(black_box("what is the weather"));
            let _intent = black_box(engine.best_intent());
            let _defs_len = black_box(grok_cli::tools::registry::get_full_tool_definitions().len());
            black_box(())
        })
    });
}

// ---------------------------------------------------------------------------
// Criterion configuration
// ---------------------------------------------------------------------------
criterion_group! {
    name = perf;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(2))
        .sample_size(50)
        .warm_up_time(Duration::from_millis(300));
    targets = bench_perf_helpers,
              bench_tool_registry,
              bench_message_construction,
              bench_bayesian_and_router,
              bench_light_turn_micro
}

criterion_main!(perf);

// ---------------------------------------------------------------------------
// Small sync helper so we can benchmark the Bayesian part of routing without
// pulling in full async machinery in the bench.
// This is intentionally not part of the public Router API.
// ---------------------------------------------------------------------------
impl Router {
    #[doc(hidden)]
    pub fn route_sync_for_bench(&mut self, input: &str) -> Option<String> {
        // Mirror the sync parts of the real route() method
        self.bayes.update_from_text(input);
        self.bayes.best_intent()
    }
}