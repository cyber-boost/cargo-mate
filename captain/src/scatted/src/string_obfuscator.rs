use std::collections::HashMap;
use rand::RngCore;
pub struct StringObfuscator {
    strings: HashMap<u32, Vec<u8>>,
    key: [u8; 32],
}
impl StringObfuscator {
    pub fn new() -> Self {
        let mut key = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        Self {
            strings: HashMap::new(),
            key,
        }
    }
    pub fn add_string(&mut self, s: &str) -> u32 {
        let id = rand::random::<u32>();
        let encrypted = self.encrypt_string(s.as_bytes());
        self.strings.insert(id, encrypted);
        id
    }
    fn encrypt_string(&self, data: &[u8]) -> Vec<u8> {
        data.iter()
            .enumerate()
            .map(|(i, &b)| b ^ self.key[i % self.key.len()] ^ (i as u8))
            .collect()
    }
    pub fn generate_decryptor(&self) -> String {
        let mut entries = String::new();
        for (id, enc_data) in &self.strings {
            entries.push_str(&format!("        {} => vec!{:?},\n", id, enc_data));
        }
        format!(
            r#"
            fn decrypt_str(id: u32) -> String {{
                const KEY: [u8; 32] = {:?};

                let encrypted: Vec<u8> = match id {{
{}{}                    _ => return String::new(),
                }};

                let decrypted: Vec<u8> = encrypted
                    .iter()
                    .enumerate()
                    .map(|(i, &b)| b ^ KEY[i % KEY.len()] ^ (i as u8))
                    .collect();

                String::from_utf8(decrypted).unwrap_or_default()
            }}
            "#,
            self.key, entries, if entries.is_empty() { "" } else { "        " }
        )
    }
    pub fn obfuscate_code(&mut self, code: &str) -> String {
        let mut result = code.to_string();
        let strings_to_obfuscate = [
            ("PROTECT_KEY not set", "Missing protection key"),
            ("decrypt failed", "Decryption error"),
            ("write failed", "Write error"),
            ("spawn failed", "Execution error"),
            ("Integrity check failed", "Validation error"),
        ];
        for (original, _) in &strings_to_obfuscate {
            let id = self.add_string(original);
            result = result
                .replace(&format!(r#""{}""#, original), &format!("decrypt_str({})", id));
        }
        format!("{}\n\n{}", self.generate_decryptor(), result)
    }
}