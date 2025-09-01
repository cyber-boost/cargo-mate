use anyhow::{Result, Context};
use std::path::PathBuf;
use std::process::Command;
pub struct CaptainBinary {
    pub platform: String,
    pub architecture: String,
    pub data: Vec<u8>,
}
pub fn get_captain_binary_for_current_platform() -> Result<CaptainBinary> {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let platform = match os {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return Err(anyhow::anyhow!("Unsupported operating system: {}", os)),
    };
    let architecture = match arch {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return Err(anyhow::anyhow!("Unsupported architecture: {}", arch)),
    };
    Ok(CaptainBinary {
        platform: platform.to_string(),
        architecture: architecture.to_string(),
        data: vec![0; 1024],
    })
}
pub fn extract_and_execute_captain(args: &[&str]) -> Result<std::process::Output> {
    let captain_path = crate::captain::captain_status::find_captain_binary()
        .context("Failed to find captain binary")?;
    let output = Command::new(&captain_path)
        .args(args)
        .output()
        .context("Failed to execute captain binary")?;
    Ok(output)
}