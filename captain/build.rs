use std::process::Command;

fn main() {
    // Check if we have a C compiler available
    let has_cc = Command::new("cc").arg("--version").output().is_ok();
    let has_gcc = Command::new("gcc").arg("--version").output().is_ok();

    if !has_cc && !has_gcc {
        println!("cargo:warning=🚨 C compiler not found!");
        println!("cargo:warning=");
        println!("cargo:warning=The 'cargo-mate' crate requires a C compiler to compile dependencies.");
        println!("cargo:warning=");
        println!("cargo:warning= Quick fix - run this first:");
        println!("cargo:warning=   curl -sSL https://get.cargo.do/install.sh | bash");
        println!("cargo:warning=");
        println!("cargo:warning=📦 Or install build tools manually:");
        println!("cargo:warning=   Ubuntu/Debian: sudo apt install build-essential");
        println!("cargo:warning=   CentOS/RHEL:   sudo yum groupinstall 'Development Tools'");
        println!("cargo:warning=   Arch Linux:    sudo pacman -S base-devel");
        println!("cargo:warning=   macOS:         brew install gcc");
        println!("cargo:warning=");
        println!("cargo:warning=Then retry: cargo install cargo-mate");
        println!("cargo:warning=");
        println!("cargo:warning=Alternatively, use the direct download:");
        println!("cargo:warning=   curl -sSL https://get.cargo.do/install.sh | bash");

        // Don't fail the build - let the dependency compilation fail with its own error
        // This way users see our helpful message first, then the actual compiler error
    }

    // Re-run build script if these environment variables change
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CFLAGS");
}
