use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/scrubme/");
    println!("cargo:rerun-if-changed=src/scrubme/Cargo.toml");
    
    // Check if we're building the scrub crate
    if env::var("CARGO_PKG_NAME").unwrap() == "scrub" {
        // This is the scrub crate being built
        return Ok(());
    }
    
    // Build the scrub crate first
    let scrub_dir = Path::new("src/scrubme");
    if !scrub_dir.exists() {
        eprintln!("Scrub directory not found, skipping binary build");
        return Ok(());
    }

    // Check if this is actually a cargo project
    let cargo_toml = scrub_dir.join("Cargo.toml");
    if !cargo_toml.exists() {
        eprintln!("Cargo.toml not found in scrub directory, skipping binary build");
        return Ok(());
    }

    println!("Building scrub crate...");

    // Change to scrub directory and build
    let current_dir = env::current_dir()?;
    env::set_current_dir(scrub_dir)?;

    // Build the scrub crate
    let status = Command::new("cargo")
        .args(["build", "--release"])
        .status()?;

    if !status.success() {
        return Err("Failed to build scrub crate".into());
    }
    
    // Return to original directory
    env::set_current_dir(&current_dir)?;
    
    // Set up the paths for the scrub binary
    let scrub_bin_path = if cfg!(windows) {
        scrub_dir.join("target/release/scrub.exe")
    } else {
        scrub_dir.join("target/release/scrub")
    };
    
    println!("Scrub binary path: {}", scrub_bin_path.display());
    
    // Create symlink to ~/.shipwreck/bin directory
    let home = env::var("HOME").unwrap_or_else(|_| ".".to_string());
    let shipwreck_bin_dir = PathBuf::from(&home).join(".shipwreck").join("bin");
    
    // Create the directory if it doesn't exist
    fs::create_dir_all(&shipwreck_bin_dir)?;
    
    let target_link = shipwreck_bin_dir.join("scrub");
    
    // Remove existing symlink if it exists
    if target_link.exists() {
        if cfg!(unix) {
            // On Unix, we can check if it's a symlink
            let metadata = fs::symlink_metadata(&target_link)?;
            if metadata.file_type().is_symlink() {
                fs::remove_file(&target_link)?;
            } else {
                fs::remove_file(&target_link)?;
            }
        } else {
            // On Windows, just try to remove the file
            fs::remove_file(&target_link)?;
        }
    }
    
    // Create symlink
    println!("Creating symlink from {} to {}", scrub_bin_path.display(), target_link.display());
    
    // Try to create symlink (may fail if it exists)
    let symlink_result = {
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&scrub_bin_path, &target_link)
        }
        
        #[cfg(windows)]
        {
            // Windows requires different symlink functions for files vs directories
            if fs::metadata(&scrub_bin_path)?.is_dir() {
                std::os::windows::fs::symlink_dir(&scrub_bin_path, &target_link)
            } else {
                std::os::windows::fs::symlink_file(&scrub_bin_path, &target_link)
            }
        }
    };
    
    // Handle the case where the symlink already exists
    if let Err(e) = symlink_result {
        if e.kind() == std::io::ErrorKind::AlreadyExists {
            // Remove the existing file/symlink and try again
            fs::remove_file(&target_link)?;
            
            // Try to create the symlink again
            #[cfg(unix)]
            std::os::unix::fs::symlink(&scrub_bin_path, &target_link)?;
            
            #[cfg(windows)]
            {
                // Windows requires different symlink functions for files vs directories
                if fs::metadata(&scrub_bin_path)?.is_dir() {
                    std::os::windows::fs::symlink_dir(&scrub_bin_path, &target_link)?;
                } else {
                    std::os::windows::fs::symlink_file(&scrub_bin_path, &target_link)?;
                }
            }
        } else {
            // If it's some other error, propagate it
            return Err(e.into());
        }
    }
    
    println!("Symlink created successfully.");
    
    Ok(())
}