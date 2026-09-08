//! HOH CLI Commands (Task 297.13 + upgrades)
//
//! Enhanced CLI for HOH outer loop.
//! Supports: start, status, last, history, simulate, apply [--dry-run]

use crate::hoh::HOHManager;
use crate::hoh::persistence;
use crate::hoh::state::IterationState;
use std::path::PathBuf;

pub async fn handle_hoh_command(sub: &str, simulation: bool) {
    let data_dir = PathBuf::from(".grok/hoh");
    let mut manager = HOHManager::new(data_dir.clone());
    manager.config.simulation_mode = simulation;

    let parts: Vec<&str> = sub.split_whitespace().collect();
    let cmd = parts.first().copied().unwrap_or("");
    let rest = &parts[1..];

    match cmd {
        "start" | "run" => {
            println!("[HOH] Starting iteration...");
            match manager.run_iteration().await {
                Ok(state) => {
                    println!("[HOH] Iteration {} completed: {:?}", state.iteration_id, state.status);
                    print_iteration_summary(&state);
                }
                Err(e) => eprintln!("[HOH] Error: {}", e),
            }
        }
        "status" => {
            println!("[HOH] === HOH Status ===");
            // Show persisted history
            match persistence::list_iterations(&data_dir) {
                Ok(ids) => {
                    println!("Persisted iterations: {} total", ids.len());
                    if let Some(&latest_id) = ids.last() {
                        if let Ok(Some(state)) = persistence::load_iteration(&data_dir, latest_id) {
                            println!("Latest completed: iteration {}", latest_id);
                            print_iteration_summary(&state);
                        }
                    }
                }
                Err(e) => println!("Could not list iterations: {}", e),
            }

            // Current in-memory
            if let Some(current) = &manager.current_iteration {
                println!("\nIn-memory current iteration: {}", current.iteration_id);
                print_iteration_summary(current);
            } else {
                println!("\nNo current iteration loaded in memory.");
            }

            println!("Simulation mode: {}", simulation);
        }
        "last" => {
            println!("[HOH] === Last Iteration ===");
            match persistence::load_latest_iteration(&data_dir) {
                Ok(Some(state)) => {
                    println!("Iteration {} (status: {:?})", state.iteration_id, state.status);
                    print_iteration_summary(&state);
                    println!("\n--- Plan goals ---");
                    if let Some(plan) = &state.plan {
                        for g in &plan.goals {
                            println!("  - {}", g);
                        }
                    }
                    println!("\n--- Patches ({} total) ---", state.patches.len());
                    for p in state.patches.iter().take(5) {
                        println!("  {} | {} files | {}", p.id, p.files_changed.len(), p.source);
                    }
                    if state.patches.len() > 5 {
                        println!("  ... and {} more", state.patches.len() - 5);
                    }
                }
                Ok(None) => println!("No completed iterations found in persistence."),
                Err(e) => eprintln!("Error loading last: {}", e),
            }
        }
        "history" => {
            println!("[HOH] === Iteration History ===");
            match persistence::list_iterations(&data_dir) {
                Ok(ids) => {
                    if ids.is_empty() {
                        println!("No persisted iterations yet.");
                    } else {
                        for id in ids.iter().rev().take(10) {
                            if let Ok(Some(state)) = persistence::load_iteration(&data_dir, *id) {
                                let summary = state.summary.as_deref().unwrap_or("no summary");
                                println!("  {:04} | {:?} | patches={} | {}", id, state.status, state.patches.len(), summary.chars().take(60).collect::<String>());
                            }
                        }
                        if ids.len() > 10 {
                            println!("  ... ({} more)", ids.len() - 10);
                        }
                    }
                }
                Err(e) => eprintln!("Error listing history: {}", e),
            }
        }
        "apply" => {
            let dry_run = rest.contains(&"--dry-run") || simulation;
            println!("[HOH] Apply command (dry_run={})", dry_run);

            // For now, run a fresh iteration in dry-run mode and show what would apply
            manager.config.simulation_mode = true; // force safe
            match manager.run_iteration().await {
                Ok(state) => {
                    println!("[HOH] Generated {} patches in this run.", state.patches.len());
                    for p in &state.patches {
                        println!("  Would apply: {} → {:?}", p.id, p.files_changed);
                        if dry_run {
                            println!("    (dry-run: content preview length = {})", p.content_to_apply().len());
                        }
                    }
                    println!("[HOH] (Note: actual apply still gated by tests + meta score in FullAuto mode)");
                }
                Err(e) => eprintln!("[HOH] Apply simulation error: {}", e),
            }
        }
        "simulate" => {
            println!("[HOH] Running simulation...");
            let state = crate::hoh::simulation::run_simulation(1).await;
            println!("[HOH] Simulation result: {:?}", state.status);
            print_iteration_summary(&state);
        }
        _ => {
            println!("Unknown hoh subcommand: '{}'", cmd);
            println!("Available: start, status, last, history, apply [--dry-run], simulate");
        }
    }
}

fn print_iteration_summary(state: &IterationState) {
    println!("  ID: {}", state.iteration_id);
    println!("  Status: {:?}", state.status);
    if let Some(s) = &state.summary {
        println!("  Summary: {}", s);
    }
    println!("  Patches: {}", state.patches.len());
    println!("  Evaluations: {}", state.evaluations.len());
    if let Some(plan) = &state.plan {
        println!("  Materialized tasks: {}", plan.materialized_task_ids.len());
        println!("  Patch stubs (361.3 C): {}", plan.generated_patch_stubs.len());
    }
}