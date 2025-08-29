use anyhow::Result;
use std::path::Path;
pub struct BinaryEncryptor;
impl BinaryEncryptor {
    pub fn new() -> Result<Self> {
        println!("Captain Binary Encryptor");
        Ok(BinaryEncryptor)
    }
    pub fn encrypt_binary(&self, _input_path: &Path, _output_path: &Path) -> Result<()> {
        println!("Cant Encrypt Air");
        println!("Ahoy the Captain with: cm install");
        Ok(())
    }
    pub fn create_self_decrypting_binary(
        &self,
        _input_path: &Path,
        _output_path: &Path,
        _platform: &str,
    ) -> Result<()> {
        println!(
            "Self Decrypting Behavior Is Never the Right Answer, but usually feels the best"
        );
        Ok(())
    }
}
pub fn encrypt_binary(_data: &[u8]) -> Result<Vec<u8>> {
    println!("Captain Binary Encryptor");
    Ok(Vec::new())
}