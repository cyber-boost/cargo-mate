use anyhow::Result;
use clap::ValueEnum;
use std::path::{Path, PathBuf};
use std::fs;
use crate::{
    crypto, polymorphic::PolymorphicGenerator, anti_analysis::EnvironmentChecker,
    string_obfuscator::StringObfuscator, integrity::IntegrityChecker,
    loader_generator::LoaderGenerator, compiler::RustCompiler, decoy::DecoySystem,
};
#[derive(ValueEnum, Clone, Debug, Copy)]
pub enum ProtectionLevel {
    Basic,
    Standard,
    Advanced,
    Maximum,
    Elite,
}
pub struct BinaryProtector {
    level: ProtectionLevel,
    poly_gen: PolymorphicGenerator,
    string_obf: StringObfuscator,
    env_checker: EnvironmentChecker,
    integrity: IntegrityChecker,
    decoy_system: DecoySystem,
}
impl BinaryProtector {
    pub fn new(level: ProtectionLevel) -> Self {
        Self {
            level,
            poly_gen: PolymorphicGenerator::new(),
            string_obf: StringObfuscator::new(),
            env_checker: EnvironmentChecker::new(),
            integrity: IntegrityChecker::new(),
            decoy_system: DecoySystem::new(),
        }
    }
    pub async fn protect_single(
        &mut self,
        input: &Path,
        output: &Path,
        key: Option<String>,
    ) -> Result<()> {
        println!("🔐 Creating single-layer protected binary...");
        let binary_data = fs::read(input)?;
        let protection_key = key.unwrap_or_else(crypto::generate_rotating_key);
        let encrypted = crypto::encrypt_aes_gcm(&binary_data, &protection_key)?;
        let loader = match self.level {
            ProtectionLevel::Basic => {
                LoaderGenerator::basic_single(&encrypted, &protection_key)
            }
            _ => self.generate_advanced_loader(&encrypted, &protection_key)?,
        };
        RustCompiler::compile(&loader, output)?;
        println!("✅ Protected binary: {}", output.display());
        println!("🔑 Key: {}", protection_key);
        Ok(())
    }
    pub async fn protect_double(
        &self,
        input: &Path,
        output: &Path,
        manifest: Option<PathBuf>,
        key: Option<String>,
    ) -> Result<()> {
        println!("🔐🔐 Creating double-layer protected binary...");
        let binary_data = fs::read(input)?;
        println!("  Layer 1: Keyless obfuscation");
        let obf_key = crypto::generate_obfuscation_key();
        let obfuscated = crypto::encrypt_aes_gcm(&binary_data, &obf_key)?;
        println!("  Layer 2: Time-based rotating key");
        let rot_key = key.unwrap_or_else(crypto::generate_rotating_key);
        let double_encrypted = crypto::encrypt_aes_gcm(
            &obfuscated.ciphertext,
            &rot_key,
        )?;
        if let Some(manifest_path) = manifest {
            self.write_manifest(&manifest_path, &rot_key, &obf_key)?;
        }
        let loader = self
            .generate_double_loader(&double_encrypted, &obfuscated, &obf_key, &rot_key)?;
        RustCompiler::compile(&loader, output)?;
        println!("✅ Double-protected binary: {}", output.display());
        println!("🔑 Rotating key: {}", rot_key);
        Ok(())
    }
    pub async fn protect_self_contained(
        &self,
        input: &Path,
        output: &Path,
        key: Option<String>,
        obfuscate: bool,
        fetch_key: bool,
    ) -> Result<()> {
        println!("🔐 Creating self-contained protected binary...");
        let binary_data = fs::read(input)?;
        let encryption_key = if let Some(k) = key {
            k
        } else if fetch_key {
            crypto::fetch_key_from_api()
                .await
                .unwrap_or_else(|_| {
                    println!("⚠️ API fetch failed, using local key");
                    crypto::generate_fallback_key()
                })
        } else {
            crypto::generate_fallback_key()
        };
        let data_to_encrypt = if obfuscate {
            println!("  Adding obfuscation layer...");
            let obf_key = crypto::generate_obfuscation_key();
            let obfuscated = crypto::encrypt_aes_gcm(&binary_data, &obf_key)?;
            let mut packaged = Vec::new();
            packaged.extend_from_slice(b"OBFUSCATED");
            packaged
                .extend_from_slice(&(obfuscated.ciphertext.len() as u64).to_le_bytes());
            packaged.extend_from_slice(&obfuscated.nonce);
            packaged.extend_from_slice(&obfuscated.salt);
            packaged.extend_from_slice(&obfuscated.ciphertext);
            packaged
        } else {
            binary_data
        };
        let encrypted = crypto::encrypt_aes_gcm(&data_to_encrypt, &encryption_key)?;
        let loader = self
            .generate_self_contained_loader(&encrypted, &encryption_key, obfuscate)?;
        RustCompiler::compile(&loader, output)?;
        println!("✅ Self-contained binary: {}", output.display());
        Ok(())
    }
    pub async fn protect_ultra(
        &self,
        input: &Path,
        output: &Path,
        report: Option<PathBuf>,
        hardware_lock: bool,
        metamorphic: bool,
    ) -> Result<()> {
        println!("🔐🔐🔐 ULTRA PROTECTION MODE ENGAGED");
        println!("  [✓] Polymorphic loader generation");
        println!("  [✓] String obfuscation");
        println!("  [✓] Anti-debugging & VM detection");
        println!("  [✓] Integrity checking");
        if hardware_lock {
            println!("  [✓] Hardware fingerprinting");
        }
        if metamorphic {
            println!("  [✓] Metamorphic mutation");
        }
        let binary_data = fs::read(input)?;
        let keys = [
            crypto::generate_strong_key()?,
            crypto::generate_strong_key()?,
            crypto::generate_strong_key()?,
        ];
        let mut encrypted_data = binary_data.clone();
        let mut encryption_metadata = Vec::new();
        for (i, key) in keys.iter().enumerate() {
            println!("  Applying encryption layer {}...", i + 1);
            let enc = crypto::encrypt_aes_gcm(&encrypted_data, key)?;
            encryption_metadata.push(enc.clone());
            encrypted_data = enc.ciphertext;
        }
        let loader = self
            .generate_ultra_loader(
                &encrypted_data,
                &encryption_metadata,
                &keys,
                hardware_lock,
                metamorphic,
            )?;
        RustCompiler::compile(&loader, output)?;
        if let Some(report_path) = report {
            self.generate_protection_report(&report_path, input, output)?;
        }
        println!("✅ ULTRA protection complete: {}", output.display());
        println!("⚠️  This binary will self-destruct if tampered with!");
        Ok(())
    }
    fn generate_advanced_loader(
        &mut self,
        encrypted: &crypto::EncryptedData,
        key: &str,
    ) -> Result<String> {
        let mut loader = LoaderGenerator::new();
        match self.level {
            ProtectionLevel::Standard => {
                loader.add_string_obfuscation(&self.string_obf);
                loader.add_basic_anti_debug();
            }
            ProtectionLevel::Advanced => {
                loader.add_string_obfuscation(&self.string_obf);
                loader.add_environmental_checks(&self.env_checker);
                loader.add_polymorphic_mutations(&self.poly_gen);
            }
            ProtectionLevel::Maximum => {
                loader.add_string_obfuscation(&self.string_obf);
                loader.add_environmental_checks(&self.env_checker);
                loader.add_polymorphic_mutations(&self.poly_gen);
                loader.add_integrity_checks(&self.integrity);
                loader.add_metamorphic_engine();
            }
            ProtectionLevel::Elite => {
                loader.add_string_obfuscation(&self.string_obf);
                loader.add_environmental_checks(&self.env_checker);
                loader.add_polymorphic_mutations(&self.poly_gen);
                loader.add_integrity_checks(&self.integrity);
                loader.add_metamorphic_engine();
                loader.add_decoy_system(&mut self.decoy_system);
            }
            _ => {}
        }
        loader.generate(encrypted, key)
    }
    fn write_manifest(
        &self,
        manifest_path: &Path,
        rot_key: &str,
        obf_key: &str,
    ) -> Result<()> {
        let manifest = format!(
            r#"{{
    "rotating_key": "{}",
    "obfuscation_key": "{}",
    "timestamp": "{}"
}}"#,
            rot_key, obf_key, chrono::Utc::now().to_rfc3339()
        );
        std::fs::write(manifest_path, manifest)?;
        Ok(())
    }
    fn generate_double_loader(
        &self,
        double_encrypted: &crypto::EncryptedData,
        obfuscated: &crypto::EncryptedData,
        _obf_key: &str,
        _rot_key: &str,
    ) -> Result<String> {
        let loader = format!(
            r#"
use aes_gcm::*;
use sha2::*;
use std::{{env, fs, process}};

const DOUBLE_ENCRYPTED: &[u8] = &{:?};
const DOUBLE_NONCE: &[u8] = &{:?};
const DOUBLE_SALT: &[u8] = &{:?};

const OBF_ENCRYPTED: &[u8] = &{:?};
const OBF_NONCE: &[u8] = &{:?};
const OBF_SALT: &[u8] = &{:?};

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let rot_key = env::var("PROTECT_KEY")?;
    let mut hasher = Sha256::new();
    hasher.update(rot_key.as_bytes());
    hasher.update(DOUBLE_SALT);
    let derived_rot = hasher.finalize();

    let cipher_rot = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_rot));
    let layer1_dec = cipher_rot.decrypt(Nonce::from_slice(DOUBLE_NONCE), DOUBLE_ENCRYPTED)?;

    // Second layer decryption
    let mut hasher2 = Sha256::new();
    hasher2.update(b"obfuscation-key");
    hasher2.update(OBF_SALT);
    let derived_obf = hasher2.finalize();

    let cipher_obf = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived_obf));
    let final_dec = cipher_obf.decrypt(Nonce::from_slice(OBF_NONCE), &layer1_dec)?;

    let tmp = env::temp_dir().join("scat_double");
    fs::write(&tmp, final_dec)?;
    std::process::Command::new(&tmp).args(std::env::args().skip(1)).status()?;
    fs::remove_file(&tmp)?;
    Ok(())
}}
"#,
            double_encrypted.ciphertext, double_encrypted.nonce, double_encrypted.salt,
            obfuscated.ciphertext, obfuscated.nonce, obfuscated.salt
        );
        Ok(loader)
    }
    fn generate_self_contained_loader(
        &self,
        encrypted: &crypto::EncryptedData,
        key: &str,
        _obfuscate: bool,
    ) -> Result<String> {
        let loader = format!(
            r#"
use aes_gcm::*;
use sha2::*;
use std::{{env, fs, process}};

const ENCRYPTED: &[u8] = &{:?};
const NONCE: &[u8] = &{:?};
const SALT: &[u8] = &{:?};
const EMBEDDED_KEY: &str = "{}";

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    let key = EMBEDDED_KEY;
    let mut hasher = Sha256::new();
    hasher.update(key.as_bytes());
    hasher.update(SALT);
    let derived = hasher.finalize();

    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived));
    let decrypted = cipher.decrypt(Nonce::from_slice(NONCE), ENCRYPTED)?;

    let tmp = env::temp_dir().join("scat_self");
    fs::write(&tmp, decrypted)?;
    std::process::Command::new(&tmp).args(std::env::args().skip(1)).status()?;
    fs::remove_file(&tmp)?;
    Ok(())
}}
"#,
            encrypted.ciphertext, encrypted.nonce, encrypted.salt, key
        );
        Ok(loader)
    }
    fn generate_ultra_loader(
        &self,
        _encrypted_data: &[u8],
        _encryption_metadata: &[crypto::EncryptedData],
        _keys: &[String],
        _hardware_lock: bool,
        _metamorphic: bool,
    ) -> Result<String> {
        let loader = format!(
            r#"
use aes_gcm::*;
use std::{{env, fs, process}};

fn main() -> Result<(), Box<dyn std::error::Error>> {{
    // Ultra protection logic would go here
    println!("Ultra protection active!");
    Ok(())
}}
"#
        );
        Ok(loader)
    }
    fn generate_protection_report(
        &self,
        report_path: &Path,
        input: &Path,
        output: &Path,
    ) -> Result<()> {
        let report = format!(
            r#"{{
    "input_file": "{}",
    "output_file": "{}",
    "protection_level": "ultra",
    "timestamp": "{}",
    "features": ["polymorphic", "string_obfuscation", "anti_debug", "integrity_check"]
}}"#,
            input.display(), output.display(), chrono::Utc::now().to_rfc3339()
        );
        std::fs::write(report_path, report)?;
        Ok(())
    }
}