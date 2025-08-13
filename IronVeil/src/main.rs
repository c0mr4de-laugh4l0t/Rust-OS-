use std::{fs, process::Command, path::Path, io::{self, Write}};
use sha2::{Sha256, Digest};

fn main() -> io::Result<()> {
    let kernel_dir = "../Nexis";
    let kernel_src_hash_file = ".kernel_src_hash";
    let kernel_bin = format!("{}/target/x86_64-nexis/debug/bootimage-nexis.bin", kernel_dir);

    let new_hash = hash_source(kernel_dir)?;
    let old_hash = fs::read_to_string(kernel_src_hash_file).unwrap_or_default();

    if new_hash != old_hash || !Path::new(&kernel_bin).exists() {
        println!("Kernel source changed — rebuilding...");
        let status = Command::new("cargo")
            .arg("bootimage")
            .current_dir(kernel_dir)
            .status()?;
        if !status.success() {
            eprintln!("Build failed.");
            return Ok(());
        }
        fs::write(kernel_src_hash_file, &new_hash)?;
        println!("Build complete.");
    } else {
        println!("No changes detected — skipping build.");
    }

    println!("Booting in QEMU...");
    Command::new("qemu-system-x86_64")
        .args([
            "-drive", &format!("format=raw,file={}", kernel_bin),
            "-serial", "stdio",
        ])
        .status()?;

    Ok(())
}

fn hash_source(dir: &str) -> io::Result<String> {
    let mut hasher = Sha256::new();
    for entry in walkdir::WalkDir::new(dir) {
        let entry = entry?;
        if entry.file_type().is_file() {
            let data = fs::read(entry.path())?;
            hasher.update(data);
        }
    }
    Ok(format!("{:x}", hasher.finalize()))
}