# IronVeil

**IronVeil** is a privacy-first, live USB operating system built from scratch in Rust with a custom kernel. Designed to protect user anonymity and security, IronVeil runs entirely from a USB drive and randomizes your IP address on each boot, inspired by privacy-focused systems like Tails. Leveraging Rust's memory safety, IronVeil aims to provide a secure, lightweight, and portable platform for users prioritizing anonymity and data protection.

## Features

- **Privacy by Design**: Automatically randomizes IP addresses on each boot using Tor (or similar anonymization protocols, TBD).
- **Live USB**: Runs entirely from a USB drive, leaving no trace on the host machine.
- **Rust-Powered Kernel**: Custom kernel written in Rust for memory safety and performance.
- **Secure Boot**: Ensures a trusted boot process with minimal attack surface.
- **Lightweight**: Optimized for low-resource devices, ideal for portable use.
- **Amnesiac System**: Ephemeral filesystem to prevent data persistence across sessions.
- (Add more features as implemented, e.g., encrypted storage, VPN support)

## Status

IronVeil is in **early development**. Current focus includes:
- Bootloader setup for live USB support
- Kernel initialization with basic memory management
- Integration of Tor or similar for IP randomization
- (Update with specific milestones or progress)

Expect breaking changes as the project evolves.

## Getting Started

### Prerequisites

To build and run IronVeil, you'll need:
- Rust (nightly) - `rustup install nightly`
- Cargo - Included with Rust
- QEMU (for emulation) - `qemu-system-x86_64`
- A POSIX-compliant system (Linux recommended for development)
- USB drive (for live system testing)
- (Optional) Tools for live USB creation, e.g., `dd` or `Ventoy`

### Building

1. Clone the repository:
   ```bash
   git clone https://github.com/c0mr4de-laugh4l0t/Rust-OS
   cd ironveil
