use anyhow::Result;
use std::process::{Command, Stdio};
pub fn run_cargo_with_wrapper(args: &[&str]) -> Result<()> {
    let mut cmd = Command::new("cargo");
    cmd.args(args).stdout(Stdio::inherit()).stderr(Stdio::inherit());
    let status = cmd.status()?;
    if !status.success() {
        return Err(anyhow::anyhow!("Cargo command failed"));
    }
    Ok(())
}