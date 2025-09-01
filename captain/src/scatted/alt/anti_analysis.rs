use std::fs;
use std::process::Command;
pub struct EnvironmentChecker;
impl EnvironmentChecker {
    pub fn is_sandboxed() -> bool {
        Self::check_vm() || Self::check_sandbox_artifacts() || Self::check_debuggers()
    }
    #[cfg(target_os = "linux")]
    fn check_vm() -> bool {
        let checks = vec![
            ("/sys/devices/virtual/dmi/id/product_name", vec!["VirtualBox", "VMware",
            "QEMU"]), ("/proc/cpuinfo", vec!["hypervisor"]), ("/proc/modules",
            vec!["vboxguest", "vmw_vmci", "vmmemctl"]),
        ];
        for (path, signatures) in checks {
            if let Ok(content) = fs::read_to_string(path) {
                for sig in signatures {
                    if content.contains(sig) {
                        return true;
                    }
                }
            }
        }
        let vm_files = vec![
            "/usr/bin/VBoxClient", "/usr/bin/vmware-toolbox-cmd", "/usr/bin/qemu-ga",
        ];
        for file in vm_files {
            if std::path::Path::new(file).exists() {
                return true;
            }
        }
        Self::check_cpuid_hypervisor()
    }
    #[cfg(target_os = "linux")]
    fn check_cpuid_hypervisor() -> bool {
        unsafe {
            let result: u32;
            std::arch::asm!(
                "mov eax, 1", "cpuid", "mov {0:e}, ecx", out(reg) result, out("eax") _,
                out("ebx") _, out("edx") _,
            );
            (result & (1 << 31)) != 0
        }
    }
    fn check_sandbox_artifacts() -> bool {
        let suspicious_users = vec!["sandbox", "virus", "malware", "sample", "test"];
        if let Ok(username) = std::env::var("USER") {
            for sus in suspicious_users {
                if username.to_lowercase().contains(sus) {
                    return true;
                }
            }
        }
        let analysis_tools = vec![
            "wireshark", "tcpdump", "ida", "x64dbg", "ollydbg", "windbg",
            "processhacker", "procmon", "autoruns", "fiddler", "httpdebugger",
            "apimonitor", "radare2", "gdb"
        ];
        #[cfg(target_os = "linux")]
        {
            if let Ok(output) = Command::new("ps").args(&["aux"]).output() {
                let procs = String::from_utf8_lossy(&output.stdout).to_lowercase();
                for tool in analysis_tools {
                    if procs.contains(tool) {
                        return true;
                    }
                }
            }
        }
        false
    }
    fn check_debuggers() -> bool {
        #[cfg(target_os = "linux")]
        {
            if let Ok(status) = fs::read_to_string("/proc/self/status") {
                for line in status.lines() {
                    if line.starts_with("TracerPid:") {
                        if let Some(pid) = line.split_whitespace().nth(1) {
                            if pid != "0" {
                                return true;
                            }
                        }
                    }
                }
            }
            if let Ok(cmdline) = fs::read_to_string("/proc/self/cmdline") {
                if cmdline.contains("gdb") {
                    return true;
                }
            }
            if std::env::var("LD_PRELOAD").is_ok() {
                return true;
            }
            if Self::scan_for_breakpoints() {
                return true;
            }
        }
        false
    }
    #[cfg(target_os = "linux")]
    fn scan_for_breakpoints() -> bool {
        unsafe {
            let main_addr = Self::scan_for_breakpoints as *const u8;
            let scan_size = 4096;
            for i in 0..scan_size {
                let byte = *main_addr.offset(i);
                if byte == 0xCC {
                    return true;
                }
            }
        }
        false
    }
    pub fn detect_single_stepping() -> bool {
        #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
        unsafe {
            let mut iterations = 0;
            let threshold = 1000;
            for _ in 0..5 {
                let start = Self::rdtsc();
                std::hint::black_box(42 * 42);
                let end = Self::rdtsc();
                let delta = end.wrapping_sub(start);
                if delta > threshold {
                    iterations += 1;
                }
            }
            iterations >= 3
        }
        #[cfg(not(any(target_arch = "x86", target_arch = "x86_64")))] false
    }
    #[cfg(any(target_arch = "x86", target_arch = "x86_64"))]
    unsafe fn rdtsc() -> u64 {
        let lo: u32;
        let hi: u32;
        std::arch::asm!("rdtsc", out("eax") lo, out("edx") hi,);
        ((hi as u64) << 32) | (lo as u64)
    }
}
pub fn generate_anti_analysis_checks() -> String {
    r#"
    // Environmental checks
    if EnvironmentChecker::is_sandboxed() {
        // Don't explicitly say why we're exiting
        std::process::exit(137);
    }
    
    if EnvironmentChecker::detect_single_stepping() {
        // Crash with random signal
        unsafe { std::ptr::null_mut::<i32>().write(42); }
    }
    
    // Periodic re-checks in random intervals
    std::thread::spawn(|| {
        loop {
            std::thread::sleep(std::time::Duration::from_secs(rand::random::<u64>() % 10 + 5));
            if EnvironmentChecker::is_sandboxed() {
                std::process::exit(1);
            }
        }
    });
    "#
        .to_string()
}