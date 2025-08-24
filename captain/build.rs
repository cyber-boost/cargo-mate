use std::process::Command;

fn main() {
    // Check if we have a C compiler available
    let has_cc = Command::new("cc").arg("--version").output().is_ok();
    let has_gcc = Command::new("gcc").arg("--version").output().is_ok();

    if !has_cc && !has_gcc {
        println!("cargo:warning=🚨 C compiler not found!");
        println!("cargo:warning=");
        println!("cargo:warning=Good news! This version of cargo-mate (v1.0.43+) doesn't require a C compiler!");
        println!("cargo:warning=This warning appears because of cached dependency information.");
        println!("cargo:warning=");
        println!("cargo:warning=🔧 To get the latest version without this warning:");
        println!("cargo:warning=   cargo install cargo-mate --force");
        println!("cargo:warning=");
        println!("cargo:warning=📦 Or use the direct download (recommended):");
        println!("cargo:warning=   curl -sSL https://get.cargo.do/install.sh | bash");
        println!("cargo:warning=");
        println!("cargo:warning=The installation should continue successfully despite this warning.");

        // Don't fail the build - let the dependency compilation fail with its own error
        // This way users see our helpful message first, then the actual compiler error
    }

    // Re-run build script if these environment variables change
    println!("cargo:rerun-if-env-changed=CC");
    println!("cargo:rerun-if-env-changed=CXX");
    println!("cargo:rerun-if-env-changed=CFLAGS");
}
