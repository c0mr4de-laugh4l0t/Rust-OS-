use std::io::{self, Write};
use std::process::Command;
use std::thread::sleep;
use std::time::Duration;
use std::path::Path;

fn main() {
    banner();
    loop {
        print!("\x1b[38;5;208mironveil>\x1b[0m ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let cmd = input.trim();

        match cmd {
            "help" => {
                println!("\x1b[38;5;208mAvailable commands:\x1b[0m");
                println!("  help    - Show this help message");
                println!("  clear   - Clear the screen");
                println!("  boot    - Build & boot the Nexis kernel in QEMU");
                println!("  exit    - Quit IronVeil CLI");
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
            }
            "boot" => {
                let kernel_path = "../Nexis/target/x86_64-nexis/debug/bootimage-nexis.bin";

                if !Path::new(kernel_path).exists() {
                    println!("\x1b[38;5;208mBoot image not found. Building Nexis kernel...\x1b[0m");
                    let status = Command::new("cargo")
                        .args(&["bootimage"])
                        .current_dir("../Nexis")
                        .status();

                    match status {
                        Ok(s) if s.success() => println!("\x1b[38;5;208mBuild successful!\x1b[0m"),
                        Ok(s) => {
                            eprintln!("\x1b[31mBuild failed with status {:?}\x1b[0m", s.code());
                            continue;
                        }
                        Err(e) => {
                            eprintln!("\x1b[31mFailed to run cargo bootimage: {}\x1b[0m", e);
                            continue;
                        }
                    }
                }

                println!("\x1b[38;5;208mBooting Nexis kernel in QEMU...\x1b[0m");
                let status = Command::new("qemu-system-x86_64")
                    .args(&[
                        "-drive", &format!("format=raw,file={}", kernel_path),
                        "-serial", "stdio",
                        "-m", "512M",
                    ])
                    .status();

                match status {
                    Ok(s) => println!("\x1b[38;5;208mQEMU exited with status: {:?}\x1b[0m", s.code()),
                    Err(e) => eprintln!("\x1b[31mFailed to run QEMU: {}\x1b[0m", e),
                }
            }
            "exit" => {
                println!("\x1b[38;5;208mExiting IronVeil CLI...\x1b[0m");
                break;
            }
            "" => {}
            _ => {
                println!("\x1b[31mUnknown command: {}\x1b[0m", cmd);
            }
        }
    }
}

fn banner() {
    let banner_lines = [
        "██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     ",
        "██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     ",
        "██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     ",
        "██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     ",
        "██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗",
        "╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝",
    ];
    print!("\x1b[2J\x1b[H");
    for line in &banner_lines {
        for c in line.chars() {
            print!("\x1b[38;5;208m{}\x1b[0m", c);
            io::stdout().flush().unwrap();
            sleep(Duration::from_millis(2));
        }
        println!();
    }
    println!();
}