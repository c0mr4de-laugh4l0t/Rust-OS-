
██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     
██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     
██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     
██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     
██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗
╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝

IronVeil OS & Nexis Kernel

IronVeil is a Rust-based privacy-focused operating system featuring the custom Nexis Kernel.
It combines a minimal, efficient kernel design with strong privacy tools such as Tor-based IP randomization, MAC spoofing, and an optional encrypted persistence mode.


Features

Custom x86_64 Nexis Kernel — written in Rust, no_std, booted via bootloader.

Preemptive multitasking — with IRQ-driven scheduler.

Physical Memory Management (PMM) — linker-based initialization.

Basic file system — simple in-memory storage with fs ls and fs cat.

On-screen VGA shell — text-based interface for interacting with the kernel.

Privacy Tools — IP randomization, MAC spoofing, password generator.


 Project Structure

.
├── LICENSE
├── README.md
├── Cargo.toml
├── Nexis/          # Kernel source code
│   └── src/
│       ├── main.rs
│       ├── interrupts.rs
│       ├── vga.rs
│       ├── kb.rs
│       ├── memory.rs
│       ├── scheduler.rs
│       ├── task.rs
│       ├── syscall.rs
│       ├── fs.rs
│       ├── alloc.rs
│       └── ...
└── IronVeil/       # OS shell & higher-level functions
    └── src/
        ├── main.rs
        └── ...



 Build & Run

Requirements:

Rust nightly toolchain

bootimage

qemu-system-x86_64


Build & run in QEMU:

cargo install bootimage
rustup override set nightly
rustup component add rust-src
cargo bootimage
qemu-system-x86_64 -drive format=raw,file=target/x86_64-nexis/debug/bootimage-nexis.bin



 Commands

From the VGA shell:

help           Show available commands
clear | cls    Clear the screen
genpass        Generate a 16-char password
ip             Generate a fake IPv4 address
mac            Generate a fake MAC address
fs ls          List available files
fs cat <file>  Print file contents
reboot         Halt the kernel (restart in QEMU)



 License

Licensed under the MIT License.



Do you want me to also write your MIT LICENSE file so you can paste it into LICENSE in the root? That way your GitHub repo will be fully ready.

