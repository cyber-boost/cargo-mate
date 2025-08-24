//! # 🚢 Cargo Mate - Source Protected Distribution
//!
//! A Rust development companion that enhances cargo with intelligent workflows,
//! state management, performance optimization, and comprehensive project monitoring.
//!
//! ## Features
//!
//! - **Journey Recording**: Record and replay complex development workflows
//! - **Anchor Points**: Save and restore complete project states with auto-update
//! - **Captain's Log**: Natural language build notes with automatic tagging
//! - **Tide Charts**: Interactive performance tracking and build analytics
//! - **Treasure Maps**: Visual dependency tree analysis
//! - **Mutiny Mode**: Override cargo restrictions when you know what you're doing
//! - **Auto-Versioning**: Automatic semantic versioning with policy support
//! - **Build Optimization**: Intelligent Cargo.toml optimization
//! - **Smart Error Parsing**: Actionable checklists from cargo errors
//!


pub use anyhow::{Result, anyhow};

/// Get the version of cargo-mate
pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Get the package name
pub fn name() -> &'static str {
    env!("CARGO_PKG_NAME")
}

/// Check if cargo-mate is properly installed
pub fn is_installed() -> bool {
    std::process::Command::new("cm")
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!version().is_empty());
    }

    #[test]
    fn test_name() {
        assert_eq!(name(), "cargo-mate");
    }
}
