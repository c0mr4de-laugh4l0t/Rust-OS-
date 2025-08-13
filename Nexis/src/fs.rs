#![no_std]

use core::fmt::Write;

pub struct FileEntry {
    pub name: &'static str,
    pub data: &'static [u8],
}

static README_TXT: &str = "IronVeil / Nexis\nPhase 4 FS stub online.\n";
static HELLO_TXT: &str = "Hello from the ramdisk!\n";

static FILES: &[FileEntry] = &[
    FileEntry { name: "readme.txt", data: README_TXT.as_bytes() },
    FileEntry { name: "hello.txt",  data: HELLO_TXT.as_bytes()  },
];

static mut MOUNTED: bool = false;

pub fn fs_init() {
    unsafe { MOUNTED = true; }
    crate::vga::vprintln!("FS: ramdisk mounted ({} files)", FILES.len());
}

pub fn mounted() -> bool {
    unsafe { MOUNTED }
}

pub fn list_files() -> usize {
    for f in FILES {
        crate::vga::vprintln!(" - {} ({} bytes)", f.name, f.data.len());
    }
    FILES.len()
}

pub fn get(name: &str) -> Option<&'static [u8]> {
    if !mounted() { return None; }
    for f in FILES {
        if f.name == name {
            return Some(f.data);
        }
    }
    None
}

pub fn print_file(name: &str) -> bool {
    if let Some(bytes) = get(name) {
        if let Ok(s) = core::str::from_utf8(bytes) {
            crate::vga::vprint!("{}", s);
            true
        } else {
            for &b in bytes {
                let _ = write!(crate::vga::VGA_WRITER.lock(), "{:02x} ", b);
            }
            crate::vga::vprintln!("");
            true
        }
    } else {
        crate::vga::vprintln!("FS: file not found: {}", name);
        false
    }
}