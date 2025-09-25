use anyhow::{Result, Context};
use std::path::PathBuf;
use std::process::Command;
pub fn find_captain_binary() -> Result<PathBuf> {
    let current_dir = std::env::current_dir()?;
    let captain_path = current_dir.join("captain");
    if captain_path.exists() {
        return Ok(captain_path);
    }
    if let Ok(output) = Command::new("which").arg("captain").output() {
        if output.status.success() {
            let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !path.is_empty() {
                return Ok(PathBuf::from(path));
            }
        }
    }
    let home_dir = dirs::home_dir().context("Could not find home directory")?;
    let shipwreck_bin = home_dir.join(".shipwreck").join("bin").join("captain");
    if shipwreck_bin.exists() {
        return Ok(shipwreck_bin);
    }
    let cargo_mate_dir = current_dir
        .join("cargo-mate")
        .join("captain")
        .join("target")
        .join("release")
        .join("captain");
    if cargo_mate_dir.exists() {
        return Ok(cargo_mate_dir);
    }
    Err(anyhow::anyhow!("Captain binary not found"))
}
pub fn is_captain_available() -> bool {
    find_captain_binary().is_ok()
}
pub fn execute_captain(args: &[&str]) -> Result<std::process::Output> {
    let captain_path = find_captain_binary()?;
    let output = Command::new(&captain_path)
        .args(args)
        .output()
        .context("Failed to execute captain binary")?;
    Ok(output)
}