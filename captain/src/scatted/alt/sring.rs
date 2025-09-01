use std::collections::HashMap;
pub struct StringObfuscator {
    strings: HashMap<u32, Vec<u8>>,
    key: [u8; 32],
}
impl StringObfuscator {
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut key);
        Self {
            strings: HashMap::new(),
            key,
        }
    }
    pub fn add_string(&mut self, s: &str) -> u32 {
        let id = rand::random::<u32>();
        let encrypted = self.xor_encrypt(s.as_bytes());
        self.strings.insert(id, encrypted);
        id
    }
    fn xor_encrypt(&self, data: &[u8]) -> Vec<u8> {
        data.iter().enumerate().map(|(i, &b)| b ^ self.key[i % self.key.len()]).collect()
    }
    pub fn generate_decryptor(&self) -> String {
        let mut entries = String::new();
        for (id, enc_data) in &self.strings {
            entries.push_str(&format!("            {} => vec!{:?},\n", id, enc_data));
        }
        format!(
            r#"
            fn decrypt_str(id: u32) -> String {{
                const KEY: [u8; 32] = {:?};
                let encrypted = match id {{
{}                    _ => panic!("invalid string id"),
                }};
                
                let decrypted: Vec<u8> = encrypted
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| b ^ KEY[i % KEY.len()])
                    .collect();
                    
                String::from_utf8(decrypted).unwrap()
            }}
            "#,
            self.key, entries
        )
    }
}
pub fn obfuscate_loader_strings(loader_code: String) -> String {
    let mut obf = StringObfuscator::new();
    let key_error_id = obf.add_string("PROTECT_KEY not set");
    let decrypt_error_id = obf.add_string("decrypt failed");
    let write_error_id = obf.add_string("write failed");
    let mut result = loader_code;
    result = result
        .replace(
            r#"expect("PROTECT_KEY not set")"#,
            &format!(r#"expect(&decrypt_str({}))"#, key_error_id),
        );
    result = result
        .replace(
            r#"expect("decrypt failed")"#,
            &format!(r#"expect(&decrypt_str({}))"#, decrypt_error_id),
        );
    format!("{}\n{}", obf.generate_decryptor(), result)
}