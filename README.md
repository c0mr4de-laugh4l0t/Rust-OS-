# Nexis Kernel

```
 .S_sSSs      sSSs   .S S.    .S    sSSs 
.SS~YS%%b    d%%SP  .SS SS.  .SS   d%%SP 
S%S   `S%b  d%S'    S%S S%S  S%S  d%S'   
S%S    S%S  S%S     S%S S%S  S%S  S%|    
S%S    S&S  S&S     S%S S%S  S&S  S&S    
S&S    S&S  S&S_Ss   SS SS   S&S  Y&Ss   
S&S    S&S  S&S~SP    S_S    S&S  `S&&S  
S&S    S&S  S&S      SS~SS   S&S    `S*S 
S*S    S*S  S*b     S*S S*S  S*S     l*S 
S*S    S*S  S*S.    S*S S*S  S*S    .S*P 
S*S    S*S   SSSbs  S*S S*S  S*S  sSS*S  
S*S    SSS    YSSP  S*S S*S  S*S  YSS'   
SP                  SP       SP          
Y                   Y        Y           

```
**Nexis** is a Rust-based kernel built for **safety, modularity, and experimentation**.  
It prevents unsafe memory access by default and includes:  
- **Preemptive multitasking** with a custom scheduler  
- **Safe memory management** via a physical memory manager  
- **Task context switching** using Rust + inline assembly  
- **Basic VGA + serial output**  
- **PS/2 keyboard input**  
- **System calls & syscall dispatcher**  
- **Early filesystem support**  

---

## Development Status 🚧
All major subsystems (scheduler, filesystem, syscalls, VGA, keyboard, memory manager) are implemented.  
Currently, the project faces **build system issues** (`cargo`, `bootimage`, and target configs), which are being debugged.  
Despite this, the core kernel code is in place and actively evolving.  

---

## Project Structure
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