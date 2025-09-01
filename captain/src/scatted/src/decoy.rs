use rand::{Rng, seq::SliceRandom};
use rand::rngs::StdRng;
use rand::SeedableRng;
use std::collections::HashMap;
pub struct DecoySystem {
    rng: StdRng,
    fake_keys: HashMap<String, String>,
    honeypot_tokens: Vec<String>,
}
impl DecoySystem {
    pub fn new() -> Self {
        let mut rng = StdRng::from_entropy();
        let mut fake_keys = HashMap::new();
        let mut honeypot_tokens = Vec::new();
        let fake_key_names = vec![
            "UPX_PROTECTION_KEY", "THEMIDA_LICENSE_KEY", "VM_PROTECT_SERIAL",
            "ENIGMA_PROTECTOR_KEY", "AC_PROTECT_LICENSE", "EXEC_CRYPT_KEY",
            "MORPHINE_LICENSE", "RLPACK_SERIAL",
        ];
        for name in fake_key_names {
            let fake_key = format!(
                "{:016x}-{:016x}-{:016x}", rng.gen::< u64 > (), rng.gen::< u64 > (), rng
                .gen::< u64 > ()
            );
            fake_keys.insert(name.to_string(), fake_key);
        }
        let honeypot_vars = vec![
            "STEAM_API_KEY", "AWS_ACCESS_KEY", "GITHUB_TOKEN", "DATABASE_PASSWORD",
            "PRIVATE_KEY", "API_SECRET",
        ];
        for var in honeypot_vars {
            honeypot_tokens.push(var.to_string());
        }
        Self {
            rng,
            fake_keys,
            honeypot_tokens,
        }
    }
    pub fn generate_fake_protector_code(&mut self) -> String {
        let _fake_protectors = vec![
            self.generate_upx_style_fake(), self.generate_themida_style_fake(), self
            .generate_vmprotect_style_fake(), self.generate_enigma_style_fake(),
        ];
        let mut result = String::new();
        for (name, key) in &self.fake_keys {
            result
                .push_str(
                    &format!(
                        r#"
                // Fake protector key check for {}
                {{
                    if let Ok(fake_key) = std::env::var("{}") {{
                        if fake_key == "{}" {{
                            // This looks like a real protector key check!
                            // But it's just a decoy - the real protection is elsewhere
                            std::hint::black_box(fake_key.as_bytes());
                        }}
                    }}
                }}
            "#,
                        name, name, key
                    ),
                );
        }
        for token in &self.honeypot_tokens {
            result
                .push_str(
                    &format!(
                        r#"
                // Honeypot: accessing {} would trigger detection
                {{
                    let _honeypot_{} = std::env::var("{}").is_ok();
                    std::hint::black_box(_honeypot_{});
                }}
            "#,
                        token, token.to_lowercase(), token, token.to_lowercase()
                    ),
                );
        }
        result
            .push_str(
                &format!(
                    r#"
            // This code was protected by UPX 3.96 and Themida 2.4.6.0
            // Hardware-locked to CPU: {}, MAC: {}, Disk: {}
            // Do not attempt to reverse engineer - licensed to {}
        "#,
                    format!("{:016x}", self.rng.gen::< u64 > ()),
                    format!("{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}", self.rng.gen::<
                    u8 > (), self.rng.gen::< u8 > (), self.rng.gen::< u8 > (), self.rng
                    .gen::< u8 > (), self.rng.gen::< u8 > (), self.rng.gen::< u8 > ()),
                    self.generate_fake_disk_serial(), self.generate_fake_company_name()
                ),
            );
        result
    }
    fn generate_upx_style_fake(&mut self) -> String {
        format!(
            r#"
            // UPX-style fake compression header
            {{
                const UPX_MAGIC: u32 = 0x{};
                const UPX_VERSION: u16 = {};

                // Fake UPX unpacking routine
                let mut fake_buffer = vec![0u8; {}];
                for i in 0..fake_buffer.len() {{
                    fake_buffer[i] = ((i * {}) as u8) ^ {};
                }}

                // Fake NRV compression detection
                if fake_buffer.len() > {} {{
                    // This looks like real UPX NRV decompression!
                    std::hint::black_box(fake_buffer);
                }}
            }}
        "#,
            format!("{:08x}", self.rng.gen::< u32 > ()), self.rng.gen_range(0x300
            ..0x400), self.rng.gen_range(1000..10000), self.rng.gen_range(1..255), self
            .rng.gen_range(0..255), self.rng.gen_range(500..2000)
        )
    }
    fn generate_themida_style_fake(&mut self) -> String {
        format!(
            r#"
            // Themida-style fake protection
            {{
                const THEMIDA_SECTIONS: &[&str] = &["{}", "{}", "{}"];

                // Fake Themida virtual machine detection
                let mut vm_code = [0u32; {}];
                for i in 0..vm_code.len() {{
                    vm_code[i] = {} ^ (i as u32 * {});
                }}

                // Fake Themida mutation engine
                if vm_code.iter().sum::<u32>() % {} == 0 {{
                    // This looks like real Themida VM code!
                    std::hint::black_box(vm_code);
                }}
            }}
        "#,
            self.generate_fake_section_name(), self.generate_fake_section_name(), self
            .generate_fake_section_name(), self.rng.gen_range(10..50), self.rng.gen::<
            u32 > (), self.rng.gen_range(1..100), self.rng.gen_range(2..10)
        )
    }
    fn generate_vmprotect_style_fake(&mut self) -> String {
        format!(
            r#"
            // VMProtect-style fake protection
            {{
                const VMP_SECTIONS: &[&str] = &["{}", "{}", "{}"];

                // Fake VMProtect SDK calls
                let sdk_version = {};
                let mut protection_flags = {}u32;

                // Fake ultra/ultra+ mode detection
                if sdk_version >= {} {{
                    protection_flags |= {};
                    // This looks like real VMProtect ultra mode!
                }}

                std::hint::black_box(protection_flags);
            }}
        "#,
            self.generate_fake_section_name(), self.generate_fake_section_name(), self
            .generate_fake_section_name(), self.rng.gen_range(2000..4000), self.rng
            .gen::< u32 > (), self.rng.gen_range(3000..3500), self.rng.gen::< u32 > ()
        )
    }
    fn generate_enigma_style_fake(&mut self) -> String {
        format!(
            r#"
            // Enigma Protector-style fake protection
            {{
                // Fake Enigma SDK calls
                let mut enigma_data = [{}u8; {}];
                let key = {}u32;

                // Fake enigma encryption/decryption
                for i in 0..enigma_data.len() {{
                    enigma_data[i] = enigma_data[i].wrapping_add((key >> (i % 32)) as u8);
                }}

                // Fake enigma virtual machine
                if enigma_data.iter().fold(0u8, |acc, &x| acc.wrapping_add(x)) == {} {{
                    // This looks like real Enigma VM code!
                    std::hint::black_box(enigma_data);
                }}
            }}
        "#,
            self.rng.gen_range(0..255), self.rng.gen_range(50..200), self.rng.gen::< u32
            > (), self.rng.gen_range(0..255)
        )
    }
    fn generate_fake_company_name(&mut self) -> String {
        let companies = vec![
            "Microsoft Corporation", "Valve Software", "Epic Games Inc",
            "Electronic Arts", "Ubisoft Entertainment", "Activision Blizzard",
            "Rockstar Games",
        ];
        companies.choose(&mut self.rng).unwrap().to_string()
    }
    pub fn generate_fake_hardware_binding(&mut self) -> String {
        format!(
            r#"
            // Fake hardware binding - looks real but doesn't actually lock
            {{
                // Fake CPU ID binding
                let fake_cpu_id = 0x{:016x}u64;
                let current_cpu = {{
                    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
                    {{
                        let mut cpu_id: u64;
                        unsafe {{
                            std::arch::asm!(
                                "mov rax, 1",
                                "cpuid",
                                out("rax") cpu_id,
                                out("rbx") _,
                                out("rcx") _,
                                out("rdx") _,
                            );
                        }}
                        cpu_id
                    }}
                    #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))]
                    {{
                        0u64
                    }}
                }};

                // Fake MAC address binding
                let fake_mac = [0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}, 0x{:02x}];
                let machine_mac = [0x00, 0x00, 0x00, 0x00, 0x00, 0x00]; // Would read real MAC

                // Fake disk serial binding
                let fake_disk_serial = "{}";
                let machine_disk = "FAKE_DISK_SERIAL"; // Would read real serial

                // Fake binding check
                if current_cpu == fake_cpu_id {{
                    std::hint::black_box(fake_mac);
                }}

                std::hint::black_box((fake_disk_serial, machine_disk));
            }}
        "#,
            self.rng.gen::< u64 > (), self.rng.gen::< u8 > (), self.rng.gen::< u8 > (),
            self.rng.gen::< u8 > (), self.rng.gen::< u8 > (), self.rng.gen::< u8 > (),
            self.rng.gen::< u8 > (), self.generate_fake_disk_serial()
        )
    }
    fn generate_fake_disk_serial(&mut self) -> String {
        format!(
            "{:04X}-{:04X}-{:04X}-{:04X}", self.rng.gen::< u16 > (), self.rng.gen::< u16
            > (), self.rng.gen::< u16 > (), self.rng.gen::< u16 > ()
        )
    }
    fn generate_fake_section_name(&mut self) -> String {
        let sections = vec![
            ".text", ".data", ".rdata", ".idata", ".edata", ".rsrc", ".reloc", ".tls",
            ".pdata", ".xdata", ".bss", ".code"
        ];
        sections.choose(&mut self.rng).unwrap().to_string()
    }
    pub fn generate_fake_error_messages(&mut self) -> Vec<String> {
        vec![
            format!("This application has been protected by UPX {} and cannot be debugged.",
            self.rng.gen_range(3..5)),
            format!("Themida v{}.{} protection active. Debugging prohibited.", self.rng
            .gen_range(2..3), self.rng.gen_range(0..10)),
            format!("VMProtect {} Ultra mode enabled. Anti-debugging active.", self.rng
            .gen_range(3..4)), "Enigma Protector - Virtual Machine detected".to_string(),
            "RLPack compression - Decompression failed".to_string(),
            format!("ExecCryptor {} - Protection violation detected", self.rng
            .gen_range(2..4)),
        ]
    }
    pub fn generate_fake_function_signatures(&mut self) -> String {
        let mut result = String::new();
        let fake_functions = vec![
            "VMProtectBegin", "VMProtectEnd", "ThemidaBegin", "ThemidaEnd",
            "Enigma_Begin", "Enigma_End", "UPX_Unpacker", "RLPack_Decompress",
        ];
        for func in fake_functions {
            result
                .push_str(
                    &format!(
                        r#"
                // Fake {} marker - confuses reverse engineers
                {{
                    let fake_{}_marker = {};
                    std::hint::black_box(fake_{}_marker);
                }}
            "#,
                        func, func.to_lowercase(), self.rng.gen::< u32 > (), func
                        .to_lowercase()
                    ),
                );
        }
        result
    }
}