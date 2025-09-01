use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use std::process::Command;
use tempfile::TempDir;
pub struct RustCompiler;
impl RustCompiler {
    pub fn compile(loader_code: &str, output: &Path) -> Result<()> {
        let temp_dir = TempDir::new()?;
        let cargo_toml = r#"[package]
name = "scat_loader"
version = "1.0.0"
edition = "2021"

[dependencies]
aes-gcm = "0.10"
sha2 = "0.10"
hkdf = "0.12"
rand = "0.8"
chrono = "0.4"
hex = "0.4"
libc = "0.2"

[target.'cfg(windows)'.dependencies]
winapi = { version = "0.3", features = ["debugapi"] }

[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
strip = true
panic = "abort"
"#;
        fs::write(temp_dir.path().join("Cargo.toml"), cargo_toml)?;
        let src_dir = temp_dir.path().join("src");
        fs::create_dir(&src_dir)?;
        fs::write(src_dir.join("main.rs"), loader_code)?;
        println!("⚙️  Compiling protected loader...");
        let output_result = Command::new("cargo")
            .args(&["build", "--release", "--quiet"])
            .current_dir(temp_dir.path())
            .output()
            .context("Failed to run cargo")?;
        if !output_result.status.success() {
            let stderr = String::from_utf8_lossy(&output_result.stderr);
            anyhow::bail!("Compilation failed:\n{}", stderr);
        }
        let compiled = temp_dir
            .path()
            .join("target")
            .join("release")
            .join("scat_loader");
        fs::copy(&compiled, output).context("Failed to copy compiled binary")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(output, fs::Permissions::from_mode(0o755))?;
        }
        Ok(())
    }
}