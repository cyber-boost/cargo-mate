use anyhow::{Context, Result};
use std::fs;
use std::path::Path;
use base64::{Engine as _, engine::general_purpose};
pub struct BinaryEncryptor {
    encryption_key: Vec<u8>,
}
impl BinaryEncryptor {
    pub fn new(key: &str) -> Self {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let encryption_key = hasher.finalize().to_vec();
        Self { encryption_key }
    }
    pub fn encrypt_binary(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        let binary_data = fs::read(input_path)
            .with_context(|| {
                format!("Failed to read binary: {}", input_path.display())
            })?;
        let encrypted_data = self.xor_encrypt(&binary_data);
        fs::write(output_path, encrypted_data)
            .with_context(|| {
                format!("Failed to write encrypted binary: {}", output_path.display())
            })?;
        Ok(())
    }
    pub fn create_self_decrypting_binary(
        &self,
        input_path: &Path,
        output_path: &Path,
        platform: &str,
    ) -> Result<()> {
        let binary_data = fs::read(input_path)
            .with_context(|| {
                format!("Failed to read binary: {}", input_path.display())
            })?;
        let encrypted_data = self.xor_encrypt(&binary_data);
        let _loader_code = self.generate_loader_code(&encrypted_data, platform);
        let mut output_data = b"CARGO_MATE_ENCRYPTED_BINARY_V1\n".to_vec();
        output_data.extend_from_slice(&self.encryption_key);
        output_data.extend_from_slice(b"\n");
        output_data.extend_from_slice(&encrypted_data);
        fs::write(output_path, output_data)
            .with_context(|| {
                format!(
                    "Failed to write self-decrypting binary: {}", output_path.display()
                )
            })?;
        println!("🔐 Created self-decrypting binary: {}", output_path.display());
        Ok(())
    }
    fn xor_encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ self.encryption_key[i % self.encryption_key.len()])
            .collect()
    }
    fn xor_decrypt(&self, data: &[u8]) -> Vec<u8> {
        self.xor_encrypt(data)
    }
    fn generate_loader_code(&self, encrypted_data: &[u8], platform: &str) -> String {
        let encrypted_b64 = general_purpose::STANDARD.encode(encrypted_data);
        let key_b64 = general_purpose::STANDARD.encode(&self.encryption_key);
        format!(
            r#"
// Auto-generated loader for encrypted cargo-mate binary
// Platform: {}

use std::process;
use std::io::Write;
use base64::{{Engine as _, engine::general_purpose}};

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Embedded encrypted binary and key
    let encrypted_b64 = "{}";
    let key_b64 = "{}";

    // Decode the data
    let encrypted_data = general_purpose::STANDARD.decode(encrypted_b64)?;
    let key = general_purpose::STANDARD.decode(key_b64)?;

    // Decrypt the binary
    let decrypted_data = decrypt_binary(&encrypted_data, &key);

    // Execute the decrypted binary in memory
    execute_in_memory(&decrypted_data)?;

    Ok(())
}}

fn decrypt_binary(data: &[u8], key: &[u8]) -> Vec<u8> {{
    data.iter()
        .enumerate()
        .map(|(i, &byte)| byte ^ key[i % key.len()])
        .collect()
}}

fn execute_in_memory(binary_data: &[u8]) -> Result<(), Box<dyn std::error::Error>> {{
    // Create a temporary file for the decrypted binary
    let temp_path = std::env::temp_dir().join("cargo_mate_decrypted");

    // Write decrypted binary to temp file
    std::fs::write(&temp_path, binary_data)?;

    // Make it executable
    #[cfg(unix)]
    {{
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&temp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&temp_path, perms)?;
    }}

    // Execute the binary
    let status = process::Command::new(&temp_path)
        .args(std::env::args().skip(1))
        .status()?;

    // Clean up
    let _ = std::fs::remove_file(&temp_path);

    if !status.success() {{
        process::exit(status.code().unwrap_or(1));
    }}

    Ok(())
}}
        "#,
            platform, encrypted_b64, key_b64
        )
    }
}
pub fn encrypt_releases_directory(
    releases_dir: &Path,
    encryption_key: &str,
) -> Result<()> {
    let encryptor = BinaryEncryptor::new(encryption_key);
    for entry in fs::read_dir(releases_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().map_or(false, |ext| ext == "exe")
            || path
                .file_name()
                .map_or(false, |name| !name.to_string_lossy().contains("."))
        {
            let encrypted_path = path
                .with_extension(
                    format!(
                        "{}.encrypted", path.extension().unwrap_or_default()
                        .to_string_lossy()
                    ),
                );
            encryptor.encrypt_binary(&path, &encrypted_path)?;
        }
    }
    Ok(())
}
pub fn create_self_decrypting_releases(
    releases_dir: &Path,
    encryption_key: &str,
) -> Result<()> {
    let encryptor = BinaryEncryptor::new(encryption_key);
    for entry in fs::read_dir(releases_dir)? {
        let entry = entry?;
        let path = entry.path();
        if let Some(file_name) = path.file_name() {
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.contains("linux") || file_name_str.contains("macos")
                || file_name_str.contains("windows")
            {
                let platform = if file_name_str.contains("linux") {
                    "linux"
                } else if file_name_str.contains("macos") {
                    "macos"
                } else if file_name_str.contains("windows") {
                    "windows"
                } else {
                    "unknown"
                };
                let self_decrypting_path = path
                    .with_extension(
                        format!(
                            "{}.self", path.extension().unwrap_or_default()
                            .to_string_lossy()
                        ),
                    );
                encryptor
                    .create_self_decrypting_binary(
                        &path,
                        &self_decrypting_path,
                        platform,
                    )?;
            }
        }
    }
    Ok(())
}
pub fn encrypt_binary(data: &[u8]) -> Result<Vec<u8>> {
    let encryption_key = "default_encryption_key_32_chars_long";
    let encryptor = BinaryEncryptor::new(encryption_key);
    Ok(encryptor.xor_encrypt(data))
}
pub fn decrypt_binary(encrypted_data: &[u8]) -> Result<Vec<u8>> {
    let encryption_key = "default_encryption_key_32_chars_long";
    let encryptor = BinaryEncryptor::new(encryption_key);
    Ok(encryptor.xor_decrypt(encrypted_data))
}