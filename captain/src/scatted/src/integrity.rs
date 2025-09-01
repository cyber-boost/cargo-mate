use anyhow::Result;
use sha2::{Sha256, Digest};
use std::fs;
pub struct IntegrityChecker;
impl IntegrityChecker {
    pub fn new() -> Self {
        Self
    }
    pub fn generate_checks(&self) -> String {
        r#"
            // Integrity checking functions
            static mut CHECKSUM_VERIFIED: bool = false;
            static mut LAST_CHECK: u64 = 0;

            fn verify_self() -> bool {
                unsafe {
                    // Only check every 60 seconds
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs();
                    
                    if now - LAST_CHECK < 60 {
                        return CHECKSUM_VERIFIED;
                    }
                    
                    LAST_CHECK = now;
                }
                
                let Ok(self_path) = std::env::current_exe() else { 
                    return false; 
                };
                
                let Ok(binary) = fs::read(&self_path) else { 
                    return false; 
                };
                
                // Check for common hooks/patches
                if detect_hooks(&binary) {
                    return false;
                }
                
                unsafe { CHECKSUM_VERIFIED = true; }
                true
            }
            
            fn detect_hooks(binary: &[u8]) -> bool {
                // Look for common hook signatures (check individually to handle different lengths)
                let sig1 = b"\xE9";      // JMP rel32
                let sig2 = b"\xFF\x25";  // JMP [addr]
                let sig3 = b"\x48\xB8";  // MOV RAX, imm64

                if binary.windows(sig1.len()).any(|w| w == sig1) {
                    return true;
                }
                if binary.windows(sig2.len()).any(|w| w == sig2) {
                    return true;
                }
                if binary.windows(sig3.len()).any(|w| w == sig3) {
                    return true;
                }

                false
            }
            
            pub fn crc32(data: &[u8]) -> u32 {
                let mut crc = 0xFFFFFFFFu32;
                
                for &byte in data {
                    crc ^= byte as u32;
                    for _ in 0..8 {
                        if crc & 1 != 0 {
                            crc = (crc >> 1) ^ 0xEDB88320;
                        } else {
                            crc >>= 1;
                        }
                    }
                }
                
                !crc
            }
        "#
            .to_string()
    }
    pub fn generate_periodic_checks(&self) -> String {
        r#"
        // Spawn integrity checker thread
        spawn(|| {
            loop {
                sleep(std::time::Duration::from_secs(
                    rand::random::<u64>() % 30 + 30
                ));
                
                if !verify_self() {
                    // Subtle corruption - don't crash immediately
                    sleep(std::time::Duration::from_secs(
                        rand::random::<u64>() % 10
                    ));
                    
                    // Corrupt memory then exit
                    unsafe {
                        let ptr = rand::random::<usize>() as *mut u8;
                        std::ptr::write_volatile(ptr, 0xFF);
                    }
                }
            }
        });
        "#
            .to_string()
    }
    pub fn generate_integrity_system(
        binary_path: &std::path::Path,
    ) -> Result<String, std::io::Error> {
        let binary = fs::read(binary_path)?;
        let mut hasher = Sha256::new();
        hasher.update(&binary);
        let hash = hasher.finalize();
        let check_points: Vec<usize> = (0..10)
            .map(|_| rand::random::<usize>() % 100)
            .collect();
        Ok(
            format!(
                r#"
            mod integrity {{
                use sha2::{{Sha256, Digest}};
                use std::fs;
                
                const ORIGINAL_HASH: [u8; 32] = {:?};
                const CHECK_POINTS: [usize; {}] = {:?};
                static mut CHECK_COUNTER: usize = 0;
                
                pub fn verify_self() -> bool {{
                    let Ok(self_path) = std::env::current_exe() else {{ return false; }};
                    let Ok(binary) = fs::read(&self_path) else {{ return false; }};
                    
                    let mut hasher = Sha256::new();
                    hasher.update(&binary);
                    let hash = hasher.finalize();
                    
                    hash.as_slice() == ORIGINAL_HASH
                }}
                
                pub fn periodic_check() {{
                    unsafe {{
                        CHECK_COUNTER += 1;
                        if CHECK_POINTS.contains(&(CHECK_COUNTER % 100)) {{
                            if !verify_self() {{
                                // Corrupted - subtle misbehavior instead of crash
                                std::thread::sleep(std::time::Duration::from_secs(30));
                                std::process::exit(rand::random::<i32>());
                            }}
                        }}
                    }}
                }}
                
                // Hook detector - check if common hooking libraries are loaded
                pub fn detect_hooks() -> bool {{
                    #[cfg(target_os = "linux")]
                    {{
                        if let Ok(maps) = fs::read_to_string("/proc/self/maps") {{
                            let suspicious = ["frida", "hook", "inject", "ld_preload"];
                            for s in suspicious {{
                                if maps.to_lowercase().contains(s) {{
                                    return true;
                                }}
                            }}
                        }}
                    }}
                    false
                }}
            }}
            "#,
                hash.as_slice(), check_points.len(), check_points
            ),
        )
    }
}
pub fn generate_function_crc_checks() -> String {
    r#"
    // Calculate CRC of critical function at runtime
    fn verify_function_integrity<F>(func: F) -> bool 
    where F: Fn() 
    {
        let func_ptr = &func as *const F as *const u8;
        let mut crc = 0xFFFFFFFFu32;
        
        unsafe {
            for i in 0..256 {  // Check first 256 bytes of function
                let byte = *func_ptr.offset(i);
                crc = crc ^ (byte as u32);
                for _ in 0..8 {
                    if crc & 1 != 0 {
                        crc = (crc >> 1) ^ 0xEDB88320;
                    } else {
                        crc = crc >> 1;
                    }
                }
            }
        }
        
        // Compare with known good value (calculated at compile time)
        crc == EXPECTED_CRC
    }
    "#
        .to_string()
}