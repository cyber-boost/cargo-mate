use anyhow::Result;
use colored::Colorize;
pub fn show_help() -> Result<()> {
    println!("{}", "🚢 Cargo Mate (cm) - A Rustic Journey".bold());
    println!();
    println!("{}", "USAGE:".yellow());
    println!("  cm                      Auto-build or run default build");
    println!("  cm <command>            Run cm command or pass to cargo");
    println!();
    println!("{}", "SPECIAL COMMANDS:".yellow());
    println!("  cm wtf                  🤖 Ask CargoMate AI a question");
    println!("  cm idea                 💡 Submit an idea for Cargo Mate");
    println!("  cm journey              🎬 Record and play command sequences");
    println!("  cm anchor               ⚓ Save and restore project states");
    println!("  cm log                  📝 Captain's log for build notes");
    println!("  cm tide                 🌊 Performance tracking charts");
    println!("  cm map                  🗺️  Dependency visualization");
    println!("  cm mutiny               🏴‍☠️ Override cargo restrictions");
    println!("  cm config               ⚙️  Configuration management");
    println!("  cm version              🚢 Version management and auto-incrementing");
    println!("  cm view                 🔍 View build results and artifacts");
    println!("  cm optimize             🚀 Build performance optimization");
    println!("  cm checklist            📋 Show error/warning checklist");
    println!("  cm scrub                🧹 System-wide cargo clean");
    println!("  cm sweep                🧹 Remove debug print statements");
    println!("  cm history              📊 Show build history");
    println!("  cm install              🔧 Install shell integration");
    println!("  cm user                 👤 Show user information and license status");
    println!(
        "  cm affiliate            💰 Manage affiliate program & earning opportunities"
    );
    println!();
    println!("{}", "EXAMPLES:".yellow());
    println!("  cm journey record build-flow    # Record a build sequence");
    println!("  cm anchor save before-refactor  # Save current state");
    println!("  cm mutiny allow-warnings        # Temporarily allow warnings");
    println!("  cm map show                      # Show dependency tree");
    println!("  cm wtf er 10                     # Send recent errors to CargoMate AI");
    println!(
        "  cm wtf checklist 5               # Send 5 recent checklist items to CargoMate AI"
    );
    println!("  cm scrub run --dry-run           # Preview system-wide cargo clean");
    println!("  cm wtf ollama enable llama2      # Configure local Ollama integration");
    println!();
    println!("Run 'cm <command> --help' for more information on a command.");
    Ok(())
}