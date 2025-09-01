use anyhow::Result;
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use hkdf::Hkdf;
use sha2::{Sha256, Digest};
use rand::RngCore;
use chrono::Utc;
#[derive(Clone, Debug)]
pub struct EncryptedData {
    pub ciphertext: Vec<u8>,
    pub nonce: Vec<u8>,
    pub salt: Vec<u8>,
}
pub fn encrypt_aes_gcm(plaintext: &[u8], key: &str) -> Result<EncryptedData> {
    let mut salt = vec![0u8; 32];
    OsRng.fill_bytes(&mut salt);
    let hk = Hkdf::<Sha256>::new(Some(&salt), key.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"scat-encryption", &mut okm)
        .map_err(|_| anyhow::anyhow!("HKDF expansion failed"))?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&okm));
    let mut nonce_bytes = vec![0u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from_slice(&nonce_bytes);
    let ciphertext = cipher
        .encrypt(nonce, plaintext)
        .map_err(|_| anyhow::anyhow!("AES-GCM encryption failed"))?;
    Ok(EncryptedData {
        ciphertext,
        nonce: nonce_bytes,
        salt,
    })
}
pub fn decrypt_aes_gcm(encrypted: &EncryptedData, key: &str) -> Result<Vec<u8>> {
    let hk = Hkdf::<Sha256>::new(Some(&encrypted.salt), key.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"scat-encryption", &mut okm)
        .map_err(|_| anyhow::anyhow!("HKDF expansion failed"))?;
    let cipher = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&okm));
    let nonce = Nonce::from_slice(&encrypted.nonce);
    cipher
        .decrypt(nonce, encrypted.ciphertext.as_ref())
        .map_err(|_| anyhow::anyhow!("AES-GCM decryption failed"))
}
pub fn derive_key(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let hk = Hkdf::<Sha256>::new(Some(salt), password.as_bytes());
    let mut okm = [0u8; 32];
    hk.expand(b"scat-key-derivation", &mut okm)
        .map_err(|_| anyhow::anyhow!("HKDF key derivation failed"))?;
    Ok(okm)
}
pub fn generate_rotating_key() -> String {
    let now = Utc::now();
    let hour_slot = now.format("%Y%m%d%H").to_string();
    let mut hasher = Sha256::new();
    hasher.update(hour_slot.as_bytes());
    hasher.update(b"scat-rotation-2024");
    hex::encode(hasher.finalize())[..32].to_string()
}
pub fn generate_obfuscation_key() -> String {
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    let mut hasher = Sha256::new();
    hasher.update(b"obfuscate-");
    hasher.update(&key);
    hex::encode(hasher.finalize())[..32].to_string()
}
pub fn generate_strong_key() -> Result<String> {
    let mut key = vec![0u8; 32];
    OsRng.fill_bytes(&mut key);
    Ok(hex::encode(key))
}
pub fn generate_fallback_key() -> String {
    "default-scat-protection-key-2024".to_string()
}
pub async fn fetch_key_from_api() -> Result<String> {
    let client = reqwest::Client::new();
    let response = client
        .get("https://mate.cargo.do/overboard/key")
        .timeout(std::time::Duration::from_secs(5))
        .send()
        .await?;
    if !response.status().is_success() {
        anyhow::bail!("API returned status: {}", response.status());
    }
    let key = response.text().await?;
    if key.len() == 32 && key.chars().all(|c| c.is_ascii_hexdigit()) {
        Ok(key)
    } else {
        anyhow::bail!("Invalid key format from API")
    }
}