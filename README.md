# IronVeil + Nexis Kernel

```
██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     
██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     
██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     
██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     
██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗
╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝
```

IronVeil is a Rust-based privacy-focused OS built on the Nexis custom kernel.  
It features a TUI shell, fake IP/MAC generation, password generation, and a minimal FS interface.

---

## Project Structure
```
.
├── LICENSE
├── README.md
├── Cargo.toml
├── Nexis/          # Kernel source code
│   └── src/
│       ├── main.rs
│       ├── alloc.rs
│       ├── context.S
│       ├── fs.rs
│       ├── interrupt.rs
│       ├── kb.rs
│       ├── lib.rs
│       ├── memory.rs
│       ├── pit.rs
│       ├── process.rs
│       ├── scheduler.rs
│       ├── syscall.rs
│       ├── syscall_dispatch.rs
│       ├── task.rs
│       ├── userland.rs
│       └── vga.rs
└── IronVeil/       # OS shell & higher-level functions
    └── src/
        ├── main.rs
        └── ...
```

---

## Build & Run

### Requirements:
- Rust nightly toolchain
- `bootimage`
- `qemu-system-x86_64`

### Install & Build:
```bash
cargo install bootimage
rustup override set nightly
rustup component add rust-src
cargo bootimage
```

### Run in QEMU:
```bash
qemu-system-x86_64 -drive format=raw,file=target/x86_64-nexis/debug/bootimage-nexis.bin
```

---

## Commands (VGA Shell)
| Command         | Description                          |
|-----------------|--------------------------------------|
| `help`          | Show available commands              |
| `clear` / `cls` | Clear the screen                     |
| `genpass`       | Generate a 16-char password          |
| `ip`            | Generate a fake IPv4 address         |
| `mac`           | Generate a fake MAC address          |
| `fs ls`         | List available files                 |
| `fs cat <file>` | Print file contents                  |
| `reboot`        | Halt the kernel (restart in QEMU)    |

---

## License
This project is licensed under the MIT License – see the LICENSE file for details.