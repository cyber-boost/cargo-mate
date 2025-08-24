//! # Basic Cargo Mate Workflow
//!
//! This example demonstrates a typical development workflow using Cargo Mate.
//! While this is example code, it shows how the real cargo-mate commands work.

use std::process::Command;
use std::path::Path;

/// Example of a basic development workflow with Cargo Mate
///
/// This shows how you would use cargo-mate in a real project.
/// Note: This is demonstration code - the actual functionality
/// is provided by the cargo-mate binary.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🦀 Cargo Mate - Basic Development Workflow");
    println!("==========================================");

    // Check if cargo-mate is available
    let cm_available = Command::new("cm")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false);

    if !cm_available {
        println!("❌ cargo-mate not found. Install with:");
        println!("   cargo install cargo-mate");
        return Ok(());
    }

    println!("✅ cargo-mate is available");

    // Example workflow steps
    let workflow_steps = vec![
        ("Save current state", "cm anchor save before-refactor"),
        ("Start recording workflow", "cm journey record development-workflow"),
        ("Run cargo check", "cargo check"),
        ("Run tests", "cargo test"),
        ("Build release", "cargo build --release"),
        ("Stop recording", "cm journey stop development-workflow"),
        ("Show performance", "cm tide show"),
    ];

    println!("\n📋 Example Workflow:");
    for (step_num, (description, command)) in workflow_steps.iter().enumerate() {
        println!("{}. {}: {}", step_num + 1, description, command);
    }

    println!("\n🚀 Real Usage:");
    println!("   cm journey record my-workflow");
    println!("   cargo check");
    println!("   cargo test");
    println!("   # ... do your development ...");
    println!("   cm journey stop my-workflow");
    println!("   cm journey play my-workflow  # Replay anytime");

    Ok(())
}
