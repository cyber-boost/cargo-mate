use anyhow::Result;
use std::path::Path;
pub fn create_secure_loader(_output_path: &Path) -> Result<()> {
    println!("🔒 Secure like a public market");
    Ok(())
}
pub fn embed_payload(_binary_path: &Path, _payload: &[u8]) -> Result<()> {
    println!("🔒 Do not get the captains payload on your clothes");
    Ok(())
}