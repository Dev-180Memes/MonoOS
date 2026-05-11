use anyhow::Result;
use std::io::{self, BufRead, Write};
use std::path::Path;
use uuid::Uuid;
use chrono::Utc;

use mono_core::Intent;
use mono_capabilities::CapabilitiesRegistry;
use mono_ledger::Ledger;
use mono_inference::InferenceEngine;

#[tokio::main]
async fn main() -> Result<()> {
    let model_path = std::env::var("MONO_MODEL")
        .unwrap_or_else(|_| "~/models/Phi-3.5-mini-instruct-Q4_K_M.gguf".into());
    let model_path = shellexpand::tilde(&model_path).into_owned();

    let capabilities_dir = Path::new(
        env!("CARGO_MANIFEST_DIR")
    ).parent().unwrap()
    .join("mono-capabilities/capabilities");

    let ledger_path = std::env::var("MONO_LEDGER")
        .unwrap_or_else(|_| "./mono-ledger.db".into());

    println!("Mono OS - Stage 1 Intent Shell");
    println!("Loading capabilities...");
    let registry = CapabilitiesRegistry::load_from_dir(&capabilities_dir)?;
    let caps: Vec<_> = registry.all();
    println!("  {} capabilities loaded.", caps.len());

    println!("Opening ledger at {}...", ledger_path);
    let ledger = Ledger::open(&ledger_path).await?;

    println!("Loading model from {}...", model_path);
    println!("  (This takes 5–30 seconds on first load)");
    let engine = InferenceEngine::load(&model_path)?;
    println!("  Model ready.");
    println!();
    println!("Type a natural language instruction and press Enter.");
    println!("Type 'history' to see recent actions. Type 'exit' to quit.");
    println!("{}", "─".repeat(60));

    let stdin = io::stdin();
    loop {
        print!("\n-> ");
        io::stdout().flush()?;

        let mut line = String::new();
        stdin.lock().read_line(&mut line)?;
        let input = line.trim();

        if input.is_empty() { continue; }
        if input == "exit" || input == "quit" { break; }

        if input == "history" {
            show_history(&ledger).await?;
            continue;
        }

        let intent = Intent {
            id: Uuid::new_v4().to_string(),
            raw_text: input.to_string(),
            submitted_at: Utc::now(),
        };

        println!("  Planning...");

        let caps_ref: Vec<_> = registry.all();
        let plan = match engine.plan(&intent.raw_text, &caps_ref) {
            Ok(mut p) => {
                p.intent_id = intent.id.clone();
                p
            }
            Err(e) => {
                println!("  ✗ Planning failed: {}", e);
                continue;
            }
        };

        if plan.steps.is_empty() {
            println!("  ✗ No matching capability found for that instruction.");
            continue;
        }

        println!("  Plan: {} step(s)", plan.steps.len());
        for step in &plan.steps {
            println!("    • {} with inputs: {}", step.capability_id, step.inputs);
        }

        for step in &plan.steps {
            println!("\n  Executing: {}", step.capability_id);

            let result = registry.execute(step).await;

            // Log BEFORE displaying result (the invariant)
            ledger.log(
                &intent.id,
                &step.capability_id,
                &step.inputs,
                &result,
            ).await?;

            match &result {
                aether_core::ExecutionResult::Success { output } => {
                    println!("  ✓ {}", output);
                }
                aether_core::ExecutionResult::Failure { reason } => {
                    println!("  ✗ {}", reason);
                }
            }
        }
    }

    println!("\nGoodbye.");
    Ok(())
}

async fn show_history(ledger: &Ledger) -> Result<()> {
    let entries = ledger.last_entries(5).await?;
    if entries.is_empty() {
        println!("  No actions recorded yet.");
        return Ok(());
    }
    println!("  Last {} action(s):", entries.len());
    for e in &entries {
        let status = match &e.result {
            aether_core::ExecutionResult::Success { .. } => "✓",
            aether_core::ExecutionResult::Failure { .. } => "✗",
        };
        println!("  {} [{}] {} — inputs: {}",
            status, e.executed_at.format("%H:%M:%S"),
            e.capability_id, e.inputs_snapshot);
    }
    Ok(())
}
