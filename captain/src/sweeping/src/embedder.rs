use anyhow::Result;
use std::fs;
use std::path::Path;
#[cfg(feature = "embedded_binary")]
pub const EMBEDDED_SWEEP_BINARY: &[u8] = include_bytes!(env!("SWEEP_BINARY_PATH"));
#[cfg(not(feature = "embedded_binary"))]
pub const EMBEDDED_SWEEP_BINARY: &[u8] = b"";
pub fn extract_sweep_binary() -> Result<Vec<u8>> {
    use super::encryption::decrypt_binary;
    let encrypted_base64 = String::from_utf8(EMBEDDED_SWEEP_BINARY.to_vec())?;
    decrypt_binary(&encrypted_base64)
}
pub fn write_sweep_binary_to_temp() -> Result<std::path::PathBuf> {
    use tempfile::NamedTempFile;
    let binary_data = extract_sweep_binary()?;
    let temp_file = NamedTempFile::new()?;
    fs::write(&temp_file, binary_data)?;
    Ok(temp_file.path().to_path_buf())
}
pub fn execute_sweep_binary(args: &[&str]) -> Result<std::process::Output> {
    use std::process::Command;
    let temp_path = write_sweep_binary_to_temp()?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        fs::set_permissions(&temp_path, perms)?;
    }
    let output = Command::new(&temp_path).args(args).output()?;
    let _ = fs::remove_file(temp_path);
    Ok(output)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_binary_extraction() {
        if !EMBEDDED_SWEEP_BINARY.is_empty() {
            let result = extract_sweep_binary();
            assert!(result.is_ok());
        }
    }
}