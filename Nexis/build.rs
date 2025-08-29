use std::env;
use std::path::PathBuf;

fn main() {
    // Compile assembly (switch.S)
    println!("cargo:rerun-if-changed=src/asm/switch.S");
    cc::Build::new()
        .file("src/asm/switch.S")
        .flag_if_supported("-march=x86-64")
        .compile("switch");

    // Ensure linker script is passed to the linker
    println!("cargo:rerun-if-changed=linker.ld");
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let linker = PathBuf::from(manifest_dir).join("linker.ld");
    println!("cargo:rustc-link-arg=-T{}", linker.display());

    // Optional: allow embedding other artifacts later
}