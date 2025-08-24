//! # Wrapper Script Tests
//!
//! Tests to verify that the wrapper scripts function correctly
//! and handle different platforms and scenarios appropriately.

use std::path::Path;
use std::process::Command;

/// Test that wrapper scripts have proper shebangs and are executable
#[test]
fn test_wrapper_script_permissions() {
    let wrapper_scripts = vec![
        "sh/wrapper-linux.sh",
        "sh/wrapper-macos.sh",
        "sh/install.sh",
    ];

    for script in wrapper_scripts {
        let script_path = Path::new(script);
        assert!(script_path.exists(), "Wrapper script {} should exist", script);

        // Check shebang
        let content = std::fs::read_to_string(script_path).unwrap();
        assert!(content.starts_with("#!/bin/bash"),
               "Script {} should have proper shebang", script);

        // Check executable permission (on Unix-like systems)
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

/// Test Windows wrapper scripts exist and have proper structure
#[test]
fn test_windows_wrappers() {
    let windows_scripts = vec![
        "sh/wrapper-windows.bat",
        "sh/wrapper-windows.ps1",
    ];

    for script in windows_scripts {
        let script_path = Path::new(script);
        assert!(script_path.exists(), "Windows wrapper {} should exist", script);

        let content = std::fs::read_to_string(script_path).unwrap();
        assert!(content.len() > 0, "Script {} should not be empty", script);
    }
}

/// Test that wrapper scripts reference the correct binary names
#[test]
fn test_wrapper_binary_references() {
    let test_cases = vec![
        ("sh/wrapper-linux.sh", "cargo-mate-linux-x86_64.protected"),
        ("sh/wrapper-macos.sh", "cargo-mate-macos-x86_64.protected"),
        ("sh/wrapper-windows.bat", "cargo-mate-windows-x86_64.exe.protected"),
        ("sh/wrapper-windows.ps1", "cargo-mate-windows-x86_64.exe.protected"),
    ];

    for (script, expected_binary) in test_cases {
        let script_path = Path::new(script);
        let content = std::fs::read_to_string(script_path).unwrap();

        assert!(content.contains(expected_binary),
               "Script {} should reference binary {}", script, expected_binary);
    }
}

/// Test that wrapper scripts handle platform detection
#[test]
fn test_wrapper_platform_detection() {
    let scripts_with_detection = vec![
        "sh/wrapper-linux.sh",
        "sh/wrapper-macos.sh",
        "sh/wrapper-windows.ps1",
    ];

    for script in scripts_with_detection {
        let script_path = Path::new(script);
        let content = std::fs::read_to_string(script_path).unwrap();

        // Should have some form of architecture/platform detection
        assert!(content.contains("uname") || content.contains("$env:PROCESSOR_ARCHITECTURE") || content.contains("detect_arch"),
               "Script {} should have platform detection logic", script);
    }
}

/// Test that install.sh handles all supported platforms
#[test]
fn test_install_script_platforms() {
    let install_script = Path::new("sh/install.sh");
    let content = std::fs::read_to_string(install_script).unwrap();

    // Should handle all major platforms
    let platforms = vec!["linux", "macos", "windows"];
    for platform in platforms {
        assert!(content.contains(platform),
               "install.sh should handle {} platform", platform);
    }

    // Should handle different architectures
    let architectures = vec!["x86_64", "aarch64"];
    for arch in architectures {
        assert!(content.contains(arch),
               "install.sh should handle {} architecture", arch);
    }
}

/// Test that scripts follow consistent error handling patterns
#[test]
fn test_error_handling_patterns() {
    let scripts = vec![
        "sh/wrapper-linux.sh",
        "sh/wrapper-macos.sh",
        "sh/install.sh",
    ];

    for script in scripts {
        let script_path = Path::new(script);
        let content = std::fs::read_to_string(script_path).unwrap();

        // Should have some error handling
        assert!(content.contains("exit 1") || content.contains("exit") || content.contains("error"),
               "Script {} should have error handling", script);
    }
}

/// Test that scripts are properly documented
#[test]
fn test_script_documentation() {
    let scripts = vec![
        "sh/wrapper-linux.sh",
        "sh/wrapper-macos.sh",
        "sh/install.sh",
    ];

    for script in scripts {
        let script_path = Path::new(script);
        let content = std::fs::read_to_string(script_path).unwrap();

        // Should have some documentation/comments
        assert!(content.contains("#") || content.lines().count() > 10,
               "Script {} should be documented", script);
    }
}
