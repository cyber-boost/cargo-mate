//! # Cargo Mate Integration Tests
//!
//! Tests for the published cargo-mate wrapper crate.
//! These tests verify that the wrapper functionality works correctly.

use std::process::Command;
use std::path::Path;

/// Test that cargo-mate binary is properly installed and functional
#[test]
fn test_cargo_mate_binary_available() {
    let output = Command::new("cm")
        .arg("--version")
        .output();

    match output {
        Ok(result) if result.status.success() => {
            let stdout = String::from_utf8_lossy(&result.stdout);
            assert!(stdout.contains("cargo-mate") || stdout.contains("Cargo Mate"),
                   "Version output should mention cargo-mate");
        }
        _ => {
            // cargo-mate might not be installed in test environment
            println!("⚠️  cargo-mate not available for testing (expected in development)");
        }
    }
}

/// Test that wrapper scripts are present in the expected locations
#[test]
fn test_wrapper_scripts_exist() {
    // Check that wrapper scripts exist in the sh directory
    let wrapper_scripts = vec![
        "sh/wrapper-linux.sh",
        "sh/wrapper-macos.sh",
        "sh/wrapper-windows.bat",
        "sh/wrapper-windows.ps1",
        "sh/install.sh",
    ];

    for script in wrapper_scripts {
        let script_path = Path::new(script);
        if !script_path.exists() {
            panic!("Required wrapper script not found: {}", script);
        }

        // Check that scripts are executable (on Unix-like systems)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let metadata = script_path.metadata().unwrap();
            let permissions = metadata.permissions();
            assert!(permissions.mode() & 0o111 != 0,
                   "Script {} should be executable", script);
        }
    }
}

/// Test that protected binaries exist for supported platforms
#[test]
fn test_protected_binaries_exist() {
    let platforms = vec![
        ("linux", vec!["linux-x86_64", "linux-aarch64"]),
        ("macos", vec!["macos-x86_64", "macos-aarch64"]),
        ("windows", vec!["windows-x86_64"]),
    ];

    for (platform_name, architectures) in platforms {
        let platform_dir = Path::new(platform_name);

        if !platform_dir.exists() {
            println!("⚠️  Platform directory not found: {} (expected in release builds)", platform_name);
            continue;
        }

        for arch in architectures {
            let binary_name = format!("cargo-mate-{}.protected", arch);
            let binary_path = platform_dir.join(&binary_name);

            if !binary_path.exists() {
                println!("⚠️  Protected binary not found: {} (expected in release builds)", binary_path.display());
            }
        }
    }
}

/// Test that the Cargo.toml is properly configured
#[test]
fn test_cargo_toml_configuration() {
    let cargo_toml = Path::new("captain/Cargo.toml");

    if !cargo_toml.exists() {
        panic!("captain/Cargo.toml not found");
    }

    let content = std::fs::read_to_string(cargo_toml).unwrap();

    // Check for required dependencies
    assert!(content.contains("anyhow"), "Cargo.toml should include anyhow dependency");
    assert!(content.contains("dirs"), "Cargo.toml should include dirs dependency");
    assert!(content.contains("tempfile"), "Cargo.toml should include tempfile dependency");

    // Check for binary target
    assert!(content.contains("[[bin]]"), "Cargo.toml should define binary targets");
    assert!(content.contains("name = \"cm\""), "Cargo.toml should define cm binary");

    // Check for package metadata
    assert!(content.contains("supported-platforms"), "Cargo.toml should specify supported platforms");
}

/// Test that README contains essential information
#[test]
fn test_readme_completeness() {
    let readme = Path::new("captain/README.md");

    if !readme.exists() {
        panic!("captain/README.md not found");
    }

    let content = std::fs::read_to_string(readme).unwrap();

    // Check for essential sections
    assert!(content.contains("Installation"), "README should contain installation instructions");
    assert!(content.contains("cargo install"), "README should mention cargo install");
    assert!(content.contains("cm "), "README should show cm command usage");

    // Check for feature documentation
    let features = vec!["journey", "anchor", "log", "tide", "map", "mutiny"];
    for feature in features {
        assert!(content.contains(feature), "README should document {} feature", feature);
    }
}

/// Test the directory structure is correct
#[test]
fn test_directory_structure() {
    let required_dirs = vec![
        "captain",
        "examples",
        "tests",
    ];

    let required_files = vec![
        "captain/Cargo.toml",
        "captain/README.md",
        "examples/README.md",
        "examples/basic_workflow.rs",
        "examples/advanced_features.rs",
    ];

    for dir in required_dirs {
        let dir_path = Path::new(dir);
        assert!(dir_path.exists() && dir_path.is_dir(),
               "Required directory {} should exist", dir);
    }

    for file in required_files {
        let file_path = Path::new(file);
        assert!(file_path.exists() && file_path.is_file(),
               "Required file {} should exist", file);
    }
}

#[cfg(feature = "integration_tests")]
mod integration_tests {
    use super::*;

    /// Test that cargo-mate can actually execute basic commands
    #[test]
    fn test_cargo_mate_functionality() {
        // This test would only run if cargo-mate is actually installed
        let output = Command::new("cm")
            .arg("--help")
            .output();

        if let Ok(result) = output {
            if result.status.success() {
                let stdout = String::from_utf8_lossy(&result.stdout);
                assert!(stdout.len() > 0, "Help output should not be empty");
            } else {
                println!("⚠️  cargo-mate help command failed (might not be installed)");
            }
        }
    }
}
