use anyhow::{Context, Result};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn main() -> Result<()> {
    // Rerun build script if these files change
    println!("cargo:rerun-if-changed=dll_stub/src/lib.rs");
    println!("cargo:rerun-if-changed=dll_stub/Cargo.toml");
    println!("cargo:rerun-if-changed=dll_proxy/src/lib.rs");
    println!("cargo:rerun-if-changed=dll_proxy/Cargo.toml");

    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR")?);
    let dll_stub_dir = manifest_dir.join("dll_stub");
    let dll_proxy_dir = manifest_dir.join("dll_proxy");

    println!("cargo:warning=Building DLL stubs...");

    // Build 32-bit and 64-bit versions of dll_stub
    build_dll_for_target(&dll_stub_dir, "i686-pc-windows-msvc", "32-bit stub")?;
    build_dll_for_target(&dll_stub_dir, "x86_64-pc-windows-msvc", "64-bit stub")?;

    println!("cargo:warning=Building proxy DLLs...");
    // Build 32-bit and 64-bit versions of dll_proxy
    build_dll_for_target(&dll_proxy_dir, "i686-pc-windows-msvc", "32-bit proxy")?;
    build_dll_for_target(&dll_proxy_dir, "x86_64-pc-windows-msvc", "64-bit proxy")?;

    // Define source paths for the built DLLs
    let dll_stub_x86_src = dll_stub_dir.join("target/i686-pc-windows-msvc/release/uplay_stub.dll");
    let dll_stub_x64_src = dll_stub_dir.join("target/x86_64-pc-windows-msvc/release/uplay_stub.dll");
    let dll_proxy_x86_src = dll_proxy_dir.join("target/i686-pc-windows-msvc/release/uplay_proxy.dll");
    let dll_proxy_x64_src = dll_proxy_dir.join("target/x86_64-pc-windows-msvc/release/uplay_proxy.dll");

    // Validate the built DLLs
    validate_dll(&dll_stub_x86_src, "32-bit stub")?;
    validate_dll(&dll_stub_x64_src, "64-bit stub")?;
    validate_dll(&dll_proxy_x86_src, "32-bit proxy")?;
    validate_dll(&dll_proxy_x64_src, "64-bit proxy")?;

    // Define the list of DLLs to copy to the output directory and their target names
    let dlls_to_copy = [
        ("void_uplay_api.dll", &dll_stub_x86_src),
        ("void_uplay_api_x64.dll", &dll_stub_x64_src),
        ("dbdata.dll", &dll_proxy_x86_src), // Note: This is copying proxy DLL as dbdata stub
        ("dbdata_x64.dll", &dll_proxy_x64_src), // Note: This is copying proxy DLL as dbdata stub
        ("uplay_r1_loader.dll", &dll_proxy_x86_src),
        ("uplay_r1_loader64.dll", &dll_proxy_x64_src),
        ("upc_r1_loader.dll", &dll_proxy_x86_src),
        ("upc_r1_loader64.dll", &dll_proxy_x64_src),
        ("upc_r2_loader.dll", &dll_proxy_x86_src),
        ("upc_r2_loader64.dll", &dll_proxy_x64_src),
    ];

    // Create the 'out' directory in the project root if it doesn't exist
    let output_dir = manifest_dir.join("out");
    fs::create_dir_all(&output_dir)?;
    println!("cargo:warning=Ensured output directory exists at: {}", output_dir.display());

    // Copy the DLLs to the 'out' directory in the project root
    for (target_name, src_path) in &dlls_to_copy {
        let dst_path = output_dir.join(target_name);
        fs::copy(src_path, &dst_path)
            .context(format!("Failed to copy {} to {}", src_path.display(), dst_path.display()))?;
        println!("cargo:warning=  ✓ Copied {} to {} ({} bytes)", target_name, output_dir.display(), fs::metadata(&dst_path)?.len());
    }

    println!("cargo:warning=DLLs built and copied successfully to project root 'out' directory!");

    Ok(())
}

// Helper function to build a DLL for a specific target
fn build_dll_for_target(crate_dir: &PathBuf, target: &str, arch_name: &str) -> Result<()> {
    println!("cargo:warning=Building {} for target {}...", arch_name, target);

    let status = Command::new("cargo")
        .current_dir(crate_dir)
        .args(["build", "--release", "--target", target])
        .status()
        .context(format!("Failed to execute cargo build for target {}", target))?;

    if !status.success() {
        anyhow::bail!("Failed to compile {} for target {}", arch_name, target);
    }

    Ok(())
}

// Helper function to validate a built DLL
fn validate_dll(dll_path: &PathBuf, arch_name: &str) -> Result<()> {
    if !dll_path.exists() {
        anyhow::bail!("{} DLL not found at {:?}", arch_name, dll_path);
    }

    let metadata = fs::metadata(dll_path)
        .context(format!("Failed to read metadata for {} DLL", arch_name))?;

    if metadata.len() < 1024 { // Arbitrary minimum size check
        anyhow::bail!("{} DLL is suspiciously small ({} bytes)", arch_name, metadata.len());
    }

    let data = fs::read(dll_path)
        .context(format!("Failed to read content of {} DLL", arch_name))?;

    if &data[0..2] != b"MZ" { // Check for DOS header signature
        anyhow::bail!("{} DLL has invalid DOS signature", arch_name);
    }

    Ok(())
}