use crate::{
    crypto::EncryptedData, polymorphic::PolymorphicGenerator,
    string_obfuscator::StringObfuscator, anti_analysis::EnvironmentChecker,
    integrity::IntegrityChecker, decoy::DecoySystem,
};
pub struct LoaderGenerator {
    code_sections: Vec<String>,
    has_environmental_checks: bool,
    has_integrity_checks: bool,
    has_metamorphic: bool,
}
impl LoaderGenerator {
    pub fn new() -> Self {
        Self {
            code_sections: Vec::new(),
            has_environmental_checks: false,
            has_integrity_checks: false,
            has_metamorphic: false,
        }
    }
    pub fn add_string_obfuscation(&mut self, obf: &StringObfuscator) {
        self.code_sections.push(obf.generate_decryptor());
    }
    pub fn add_basic_anti_debug(&mut self) {
        self.code_sections
            .push(
                r#"
            #[cfg(target_os = "linux")]
            {
                if let Ok(status) = std::fs::read_to_string("/proc/self/status") {
                    for line in status.lines() {
                        if line.starts_with("TracerPid:") && !line.ends_with("0") {
                            std::process::exit(1);
                        }
                    }
                }
            }
        "#
                    .to_string(),
            );
    }
    pub fn add_environmental_checks(&mut self, checker: &EnvironmentChecker) {
        self.code_sections.push(checker.generate_checks());
        self.has_environmental_checks = true;
    }
    pub fn add_polymorphic_mutations(&mut self, _gen: &PolymorphicGenerator) {}
    pub fn add_integrity_checks(&mut self, integrity: &IntegrityChecker) {
        self.code_sections.push(integrity.generate_checks());
        self.has_integrity_checks = true;
    }
    pub fn add_metamorphic_engine(&mut self) {
        self.has_metamorphic = true;
        self.code_sections
            .push(
                r#"

            fn metamorph_self(encrypted_payload: &[u8], current_nonce: &[u8], current_salt: &[u8]) {
                if let Ok(self_path) = std::env::current_exe() {
                    // Generate new keys for next run
                    let mut rng = StdRng::from_entropy();
                    let new_salt: [u8; 32] = rng.gen();
                    let new_nonce: [u8; 12] = rng.gen();

                    // Derive new key from environment + time-based seed
                    let time_seed = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_le_bytes();

                    let mut hasher = Sha256::new();
                    hasher.update(b"scat-metamorph");
                    hasher.update(&time_seed);
                    hasher.update(&new_salt);
                    let new_key = hasher.finalize();

                    // Re-encrypt the payload with new keys
                    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&new_key));
                    if let Ok(new_encrypted) = cipher.encrypt(Nonce::from_slice(&new_nonce), encrypted_payload) {
                        // Read current binary
                        if let Ok(mut binary_data) = std::fs::read(&self_path) {
                            // Find and replace the encrypted data, nonce, and salt in the binary
                            // This is a simplified approach - real metamorphic would be much more sophisticated
                            if let Ok(new_binary) = update_binary_constants(&binary_data, encrypted_payload, &new_encrypted, current_nonce, &new_nonce, current_salt, &new_salt) {
                                let _ = std::fs::write(&self_path, new_binary);
                            }
                        }
                    }
                }
            }

            fn update_binary_constants(
                binary: &[u8],
                old_encrypted: &[u8],
                new_encrypted: &[u8],
                old_nonce: &[u8],
                new_nonce: &[u8],
                old_salt: &[u8],
                new_salt: &[u8],
            ) -> std::result::Result<Vec<u8>, Box<dyn std::error::Error>> {
                let mut new_binary = binary.to_vec();

                // Replace encrypted data
                if let Some(pos) = find_subsequence(binary, old_encrypted) {
                    new_binary.splice(pos..pos + old_encrypted.len(), new_encrypted.iter().cloned());
                }

                // Replace nonce
                if let Some(pos) = find_subsequence(&new_binary, old_nonce) {
                    new_binary.splice(pos..pos + old_nonce.len(), new_nonce.iter().cloned());
                }

                // Replace salt
                if let Some(pos) = find_subsequence(&new_binary, old_salt) {
                    new_binary.splice(pos..pos + old_salt.len(), new_salt.iter().cloned());
                }

                Ok(new_binary)
            }

            fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
                haystack.windows(needle.len()).position(|window| window == needle)
            }
        "#
                    .to_string(),
            );
    }
    pub fn add_decoy_system(&mut self, decoy_system: &mut DecoySystem) {
        self.code_sections.push(decoy_system.generate_fake_protector_code());
        self.code_sections.push(decoy_system.generate_fake_function_signatures());
    }
    pub fn generate(
        &self,
        encrypted: &EncryptedData,
        _key: &str,
    ) -> Result<String, anyhow::Error> {
        let mut loader = String::new();
        for section in &self.code_sections {
            loader.push_str(section);
            loader.push('\n');
        }
        let anti_analysis_check = if self.has_environmental_checks {
            r#"
                // Run anti-analysis checks
                if check_all() {
                    // Don't be obvious about why we're exiting
                    let exit_codes = [1, 2, 11, 13, 137, 139, 143];
                    let code = exit_codes[
                        (std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs() as usize) % exit_codes.len()
                    ];
                    std::process::exit(code as i32);
                }"#
                .to_string()
        } else {
            "".to_string()
        };
        let integrity_check = if self.has_integrity_checks {
            r#"
                // Start integrity monitoring thread
                spawn(|| {
                    loop {
                        sleep(std::time::Duration::from_secs(
                            (std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)
                                .unwrap()
                                .as_secs() % 30) + 30
                        ));

                        if !verify_self() {
                            // Subtle corruption - don't crash immediately
                            sleep(std::time::Duration::from_secs(
                                (std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() % 10)
                            ));

                            // Corrupt memory then exit
                            unsafe {
                                let ptr = (std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap()
                                    .as_secs() as usize % 1000) as *mut u8;
                                std::ptr::write_volatile(ptr, 0xFF);
                            }
                            std::process::exit(1);
                        }
                    }
                });"#
                .to_string()
        } else {
            "".to_string()
        };
        loader
            .push_str(
                &format!(
                    r#"
            use aes_gcm::{{aead::{{Aead, KeyInit}}, Aes256Gcm, Key, Nonce}};
            use sha2::{{Sha256, Digest}};
            use hkdf::Hkdf;
            use rand::{{Rng, SeedableRng, rngs::StdRng}};
            use std::{{thread::{{spawn, sleep}}}};

            const ENCRYPTED: &[u8] = &{:?};
            const NONCE: &[u8] = &{:?};
            const SALT: &[u8] = &{:?};

            fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {{
                // Run all checks
                {}

                {}

                // Start integrity monitoring if enabled
                {}

                // Decrypt
                let key = std::env::var("PROTECT_KEY")?;

                let hk = Hkdf::<Sha256>::new(Some(SALT), key.as_bytes());
                let mut okm = [0u8; 32];
                hk.expand(b"scat-encryption", &mut okm).map_err(|e| format!("HKDF expand error: {{:?}}", e))?;

                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&okm));
                let decrypted = cipher.decrypt(Nonce::from_slice(NONCE), ENCRYPTED).map_err(|e| format!("Decryption error: {{:?}}", e))?;

                // Execute
                let tmp = std::env::temp_dir().join("scat_tmp");
                std::fs::write(&tmp, decrypted)?;

                #[cfg(unix)]
                {{
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
                }}

                let status = std::process::Command::new(&tmp)
                    .args(std::env::args().skip(1))
                    .status()?;

                std::fs::remove_file(&tmp)?;

                // Trigger metamorphosis on successful execution (only if enabled)
                {}

                std::process::exit(status.code().unwrap_or(0));
            }}
        "#,
                    encrypted.ciphertext, encrypted.nonce, encrypted.salt, if self
                    .code_sections.is_empty() { "" } else { "// Checks run inline" },
                    anti_analysis_check, integrity_check, if self.has_metamorphic {
                    "if status.success() {\n                    metamorph_self(ENCRYPTED, NONCE, SALT);\n                }"
                    } else { "" }
                ),
            );
        Ok(loader)
    }
    pub fn basic_single(encrypted: &EncryptedData, _key: &str) -> String {
        format!(
            r#"
            use aes_gcm::{{aead::{{Aead, KeyInit}}, Aes256Gcm, Key, Nonce}};
            use sha2::{{Sha256, Digest}};
            use hkdf::Hkdf;

            const ENCRYPTED: &[u8] = &{:?};
            const NONCE: &[u8] = &{:?};
            const SALT: &[u8] = &{:?};
            
            fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {{
                let key = std::env::var("PROTECT_KEY")?;

                let hk = Hkdf::<Sha256>::new(Some(SALT), key.as_bytes());
                let mut okm = [0u8; 32];
                hk.expand(b"scat-encryption", &mut okm).map_err(|e| format!("HKDF expand error: {{:?}}", e))?;

                let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&okm));
                let decrypted = cipher.decrypt(Nonce::from_slice(NONCE), ENCRYPTED).map_err(|e| format!("Decryption error: {{:?}}", e))?;
                
                let tmp = std::env::temp_dir().join("scat_tmp");
                std::fs::write(&tmp, decrypted)?;
                
                #[cfg(unix)]
                {{
                    use std::os::unix::fs::PermissionsExt;
                    std::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o755))?;
                }}
                
                let status = std::process::Command::new(&tmp)
                    .args(std::env::args().skip(1))
                    .status()?;
                
                std::fs::remove_file(&tmp)?;
                std::process::exit(status.code().unwrap_or(0));
            }}
        "#,
            encrypted.ciphertext, encrypted.nonce, encrypted.salt
        )
    }
}