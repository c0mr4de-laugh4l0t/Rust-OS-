use spin::Mutex;
use alloc::vec::Vec;
use alloc::string::String;

#[derive(Clone)]
struct File {
    name: String,
    data: Vec<u8>,
}

lazy_static::lazy_static! {
    static ref FILES: Mutex<Vec<File>> = Mutex::new(Vec::new());
}

pub fn fs_init() {
    FILES.lock().push(File {
        name: "readme.txt".into(),
        data: b"Welcome to the IronVeil / Nexis FS!".to_vec(),
    });
}

pub fn list_files() {
    let files = FILES.lock();
    if files.is_empty() {
        crate::vga::vprintln!("No files.");
    } else {
        for f in files.iter() {
            crate::vga::vprintln!("{}", f.name);
        }
    }
}

pub fn read_file(name: &str) {
    let files = FILES.lock();
    if let Some(f) = files.iter().find(|x| x.name == name) {
        if let Ok(s) = core::str::from_utf8(&f.data) {
            crate::vga::vprintln!("{}", s);
        } else {
            crate::vga::vprintln!("<binary data>");
        }
    } else {
        crate::vga::vprintln!("File not found: {}", name);
    }
}

pub fn write_file(name: &str, data: &[u8]) {
    let mut files = FILES.lock();
    if let Some(f) = files.iter_mut().find(|x| x.name == name) {
        f.data = data.to_vec();
    } else {
        files.push(File {
            name: name.into(),
            data: data.to_vec(),
        });
    }
    crate::vga::vprintln!("Wrote {} bytes to {}", data.len(), name);
}