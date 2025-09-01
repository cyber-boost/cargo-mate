use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Key, Nonce,
};
use anyhow::Result;
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
const ENCRYPTION_KEY: &[u8] = b"your-256-bit-encryption-key-here-32-bytes";
pub fn encrypt_binary(data: &[u8]) -> Result<String> {
    let key = Key::<Aes256Gcm>::from_slice(ENCRYPTION_KEY);
    let cipher = Aes256Gcm::new(key);
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    let ciphertext = cipher
        .encrypt(&nonce, data)
        .map_err(|e| anyhow::anyhow!("AES encryption failed: {:?}", e))?;
    let mut combined = Vec::new();
    combined.extend_from_slice(nonce.as_slice());
    combined.extend_from_slice(&ciphertext);
    Ok(BASE64.encode(combined))
}
pub fn decrypt_binary(encrypted_data: &str) -> Result<Vec<u8>> {
    let combined = BASE64.decode(encrypted_data)?;
    if combined.len() < 12 {
        return Err(anyhow::anyhow!("Invalid encrypted data"));
    }
    let nonce = Nonce::from_slice(&combined[..12]);
    let ciphertext = &combined[12..];
    let key = Key::<Aes256Gcm>::from_slice(ENCRYPTION_KEY);
    let cipher = Aes256Gcm::new(key);
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|e| anyhow::anyhow!("AES decryption failed: {:?}", e))?;
    Ok(plaintext)
}
pub fn generate_key() -> String {
    BASE64.encode(ENCRYPTION_KEY)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_encryption_decryption() {
        let test_data = b"Hello, this is test data for encryption!";
        let encrypted = encrypt_binary(test_data).unwrap();
        let decrypted = decrypt_binary(&encrypted).unwrap();
        assert_eq!(test_data, decrypted.as_slice());
    }
}