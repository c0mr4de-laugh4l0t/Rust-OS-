IronVeil OS – README

Overview

IronVeil is a Rust-based privacy-focused operating system featuring a custom no_std kernel named Nexis. It is built from scratch for internal storage booting, privacy-first networking, and a secure, minimal user environment.

Features

Custom Bootloader: Initializes the CPU in 64-bit long mode, sets up GDT, IDT, and paging.

Nexis Kernel: Written in Rust with a custom x86_64-nexis.json target, providing memory management, interrupt handling, and preemptive multitasking.

System Core Services: Command execution engine, process manager, and TUI framework for user interaction.

Privacy Stack: Tor-based IP randomization, MAC spoofing, and optional encrypted persistence.

User Interface: Rust-colored CLI, animated ASCII banner, Neofetch-style info screen, and live system status panel.


Architecture

IronVeil follows a layered architecture:

1. Boot & Kernel – Bootloader and Nexis kernel.


2. System Core Services – Process management and shell.


3. Privacy & Network Stack – Tor routing, MAC spoofing, encryption.


4. User Experience Layer – CLI, TUI, live dashboards.



A detailed architecture diagram is available in IronVeil_architecture.tex.

Current Status

Bootloader loads Nexis kernel from internal storage.

CLI functional with ASCII banner.

TUI framework under development.

Preemptive multitasking implementation.

Encrypted persistence and full Tor routing planned.


Build Instructions

Requirements

Rust nightly toolchain

cargo-xbuild or cargo build -Zbuild-std

bootimage for kernel image building

QEMU for emulation


Steps

# Clone repository
git clone https://github.com/yourusername/ironveil.git
cd ironveil

# Build kernel
cargo build --target x86_64-nexis.json

# Run in QEMU
qemu-system-x86_64 -drive format=raw,file=target/x86_64-nexis/debug/bootimage-ironveil.bin

License

IronVeil is licensed under the MIT License.

