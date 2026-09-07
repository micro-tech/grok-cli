//! HOH CLI Commands (Task 297.13)

use crate::hoh::HOHManager;
use std::path::PathBuf;

pub async fn handle_hoh_command(sub: &str, simulation: bool) {
    let mut manager = HOHManager::new(PathBuf::from(".grok/hoh"));
    manager.config.simulation_mode = simulation;

    match sub {
        "start" | "run" => {
            println!("[HOH] Starting iteration...");
            match manager.run_iteration().await {
                Ok(state) => println!("[HOH] Iteration {} completed: {:?}", state.iteration_id, state.status),
                Err(e) => eprintln!("[HOH] Error: {}", e),
            }
        }
        "status" => {
            println!("[HOH] Current status (skeleton): {:?}", manager.current_iteration);
        }
        "simulate" => {
            println!("[HOH] Running simulation...");
            let state = crate::hoh::simulation::run_simulation(1).await;
            println!("[HOH] Simulation result: {:?}", state.status);
        }
        _ => println!("Unknown hoh subcommand. Try: start, status, simulate"),
    }
}