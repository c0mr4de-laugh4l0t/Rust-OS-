#![no_std]

extern crate alloc;

use alloc::vec::Vec;
use spin::Mutex;
use lazy_static::lazy_static;

struct FileEntry {
    name: &'static str,
    data: &'static [u8],
}

lazy_static! {
    static ref FS: Mutex<Vec<FileEntry>> = Mutex::new(Vec::new());
}

pub fn fs_init() {
    let mut fs = FS.lock();
    if !fs.is_empty() { return; }
    fs.push(FileEntry {
        name: "README.txt",
        data: b"IronVeil / Nexis\nType 'help' for commands.\n",
    });
    fs.push(FileEntry {
        name: "LICENSE.txt",
        data: b"All rights reserved. Demo in-kernel FS.\n",
    });
    fs.push(FileEntry {
        name: "motd.txt",
        data: b"Welcome to IronVeil/Nexis kernel shell.\n",
    });
}

pub fn list_files() {
    let fs = FS.lock();
    if fs.is_empty() {
        crate::vga::vprintln!("<fs empty>");
        return;
    }
    for f in fs.iter() {
        crate::vga::vprintln!("{} ({} bytes)", f.name, f.data.len());
    }
}

pub fn print_file(name: &str) {
    let fs = FS.lock();
    if let Some(f) = fs.iter().find(|e| e.name == name) {
        if let Ok(s) = core::str::from_utf8(f.data) {
            crate::vga::vprintln!("{}", s);
        } else {
            crate::vga::vprintln!("<binary file: {} bytes>", f.data.len());
        }
    } else {
        crate::vga::vprintln!("file not found: {}", name);
    }
}
```0