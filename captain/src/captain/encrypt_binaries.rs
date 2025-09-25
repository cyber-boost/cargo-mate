use anyhow::{Context, Result};
use sha2::{Sha256, Digest};
use base64::{Engine as _, engine::general_purpose};
use std::fs;
use std::path::Path;
struct BinaryEncryptor {
    encryption_key: Vec<u8>,
}
impl BinaryEncryptor {
    fn new(key: &str) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        let encryption_key = hasher.finalize().to_vec();
        Self { encryption_key }
    }
    fn xor_encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &byte)| byte ^ self.encryption_key[i % self.encryption_key.len()])
            .collect()
    }
    fn encrypt_binary(&self, input_path: &Path, output_path: &Path) -> Result<()> {
        let binary_data = fs::read(input_path)
            .with_context(|| {
                format!("Failed to read binary: {}", input_path.display())
            })?;
        let encrypted_data = self.xor_encrypt(&binary_data);
        fs::write(output_path, encrypted_data)
            .with_context(|| {
                format!("Failed to write encrypted binary: {}", output_path.display())
            })?;
        println!(
            "🔐 Encrypted: {} -> {}", input_path.display(), output_path.display()
        );
        Ok(())
    }
    fn create_self_decrypting_binary(
        &self,
        input_path: &Path,
        output_path: &Path,
        platform: &str,
        key: &str,
    ) -> Result<()> {
        let binary_data = fs::read(input_path)
            .with_context(|| {
                format!("Failed to read binary: {}", input_path.display())
            })?;
        let encrypted_data = self.xor_encrypt(&binary_data);
        let key_clone = key.to_string();
        let loader_code = format!(
            r#"#!/bin/bash
# Self-decrypting binary created by cargo-mate
set -e

# Create temporary file for decrypted binary
TEMP_BINARY=$(mktemp)
chmod +x "$TEMP_BINARY"
cleanup() {{
    rm -f "$TEMP_BINARY"
}}
trap cleanup EXIT

# Decrypt using embedded Python
python3 -c "
import sys
import hashlib
import base64

# Embedded encrypted data (base64 encoded)
encrypted_b64 = '{encrypted_b64}'

encrypted_data = base64.b64decode(encrypted_b64)

# Generate same key hash
key_hash = hashlib.sha256(b'{key}').digest()

# XOR decrypt
decrypted_data = bytearray()
for i, byte in enumerate(encrypted_data):
    decrypted_data.append(byte ^ key_hash[i % len(key_hash)])

# Write decrypted binary
with open('$TEMP_BINARY', 'wb') as f:
    f.write(decrypted_data)
" 2>/dev/null || {{
    echo "❌ Python not available for decryption"
    exit 1
}}

# Execute with all passed arguments
exec "$TEMP_BINARY" "$@"
"#,
            encrypted_b64 = general_purpose::STANDARD.encode(& encrypted_data), key =
            key_clone
        );
        fs::write(output_path, loader_code)
            .with_context(|| {
                format!(
                    "Failed to write self-decrypting binary: {}", output_path.display()
                )
            })?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(output_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(output_path, perms)?;
        }
        println!("🔐 Created self-decrypting binary: {}", output_path.display());
        Ok(())
    }
}
fn encrypt_releases_directory(releases_dir: &Path, encryption_key: &str) -> Result<()> {
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
    println!("✅ All binaries in {} encrypted successfully!", releases_dir.display());
    Ok(())
}
fn create_self_decrypting_releases(
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
                        encryption_key,
                    )?;
            }
        }
    }
    println!("✅ Self-decrypting binaries created successfully!");
    Ok(())
}