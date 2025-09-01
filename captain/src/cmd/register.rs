use anyhow::Result;
use colored::*;
use crate::cmd::init::is_build_process;
pub fn handle_register(
    license_key: Option<String>,
    status: bool,
    remaining: bool,
) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new();
    match license_manager.enforce_license("register") {
        Ok(true) => {
            println!(
                "📝 Registration: license_key={:?}, status={}, remaining={}",
                license_key, status, remaining
            );
            Ok(())
        }
        Ok(false) => Err(anyhow::anyhow!("License check failed")),
        Err(e) => Err(e),
    }
}