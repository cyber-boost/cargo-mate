use anyhow::Result;
pub fn handle_license_command(command: &str) -> Result<()> {
    let license_manager = crate::captain::license::LicenseManager::new();
    match license_manager.enforce_license(command) {
        Ok(true) => Ok(()),
        Ok(false) => Err(anyhow::anyhow!("License check failed")),
        Err(e) => Err(e),
    }
}