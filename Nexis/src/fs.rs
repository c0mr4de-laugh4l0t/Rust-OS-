use alloc::vec::Vec;
use alloc::string::String;
use spin::Mutex;
use lazy_static::lazy_static;

pub struct File {
    pub name: String,
    pub contents: String,
}

lazy_static! {
    static ref FS: Mutex<Vec<File>> = Mutex::new(Vec::new());
}

pub fn fs_init() {
    let mut fs = FS.lock();
    fs.push(File {
        name: String::from("readme.txt"),
        contents: String::from("Welcome to IronVeil Nexis FS!"),
    });
    fs.push(File {
        name: String::from("license.txt"),
        contents: String::from("All rights reserved."),
    });
}

pub fn list_files() {
    let fs = FS.lock();
    if fs.is_empty() {
        crate::vga::vprintln!("No files found.");
    } else {
        crate::vga::vprintln!("Files:");
        for file in fs.iter() {
            crate::vga::vprintln!(" - {}", file.name);
        }
    }
}

pub fn print_file(filename: &str) {
    let fs = FS.lock();
    if let Some(file) = fs.iter().find(|f| f.name == filename) {
        crate::vga::vprintln!("{}", file.contents);
    } else {
        crate::vga::vprintln!("File not found: {}", filename);
    }
}