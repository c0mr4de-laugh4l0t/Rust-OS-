
#![allow(dead_code)]

use core::{cmp, mem};
use spin::Mutex;

pub const MAX_FILES: usize = 16;
pub const MAX_NAME: usize = 32;
pub const MAX_FILE_SIZE: usize = 4096;
const STORAGE_BYTES: usize = MAX_FILES * MAX_FILE_SIZE;

#[derive(Clone, Copy)]
struct FileMeta {
    used: bool,
    name_len: u8,
    name: [u8; MAX_NAME],
    len: u32,
    slot: u16, // 0..MAX_FILES-1
}

impl FileMeta {
    const fn empty() -> Self {
        Self {
            used: false,
            name_len: 0,
            name: [0; MAX_NAME],
            len: 0,
            slot: 0,
        }
    }
}

pub enum FsError {
    NoSpace,
    NameTooLong,
    FileTooLarge,
    NotFound,
}

struct FsState {
    metas: [FileMeta; MAX_FILES],
    storage: [u8; STORAGE_BYTES],
}

impl FsState {
    const fn new() -> Self {
        Self {
            metas: [FileMeta::empty(); MAX_FILES],
            storage: [0; STORAGE_BYTES],
        }
    }

    fn slot_base(slot: usize) -> usize {
        slot * MAX_FILE_SIZE
    }

    fn find_by_name(&self, name: &str) -> Option<usize> {
        let nb = name.as_bytes();
        for i in 0..MAX_FILES {
            let m = &self.metas[i];
            if !m.used { continue; }
            let nlen = m.name_len as usize;
            if nlen == nb.len() && m.name[..nlen] == nb[..] {
                return Some(i);
            }
        }
        None
    }

    fn find_free_slot(&self) -> Option<usize> {
        for i in 0..MAX_FILES {
            if !self.metas[i].used {
                return Some(i);
            }
        }
        None
    }
}

static FS: Mutex<FsState> = Mutex::new(FsState::new());

pub fn fs_init() {
    let mut fs = FS.lock();
    // optional seed files
    let _ = write_internal(&mut fs, "readme.txt", b"Welcome to Nexis RAMFS.\nUse ls/cat/write.\n");
}

pub fn fs_write(name: &str, data: &[u8]) -> Result<usize, FsError> {
    let mut fs = FS.lock();
    write_internal(&mut fs, name, data)
}

pub fn fs_read(name: &str, out: &mut [u8]) -> Result<usize, FsError> {
    let fs = FS.lock();
    let idx = fs.find_by_name(name).ok_or(FsError::NotFound)?;
    let meta = &fs.metas[idx];
    let len = meta.len as usize;
    let base = FsState::slot_base(meta.slot as usize);
    let n = cmp::min(len, out.len());
    out[..n].copy_from_slice(&fs.storage[base..base + n]);
    Ok(n)
}

pub fn fs_len(name: &str) -> Result<usize, FsError> {
    let fs = FS.lock();
    let idx = fs.find_by_name(name).ok_or(FsError::NotFound)?;
    Ok(fs.metas[idx].len as usize)
}

pub fn fs_list(mut cb: impl FnMut(&str, usize)) {
    let fs = FS.lock();
    for i in 0..MAX_FILES {
        let m = &fs.metas[i];
        if m.used {
            let nlen = m.name_len as usize;
            let nm = unsafe { core::str::from_utf8_unchecked(&m.name[..nlen]) };
            cb(nm, m.len as usize);
        }
    }
}

fn write_internal(fs: &mut FsState, name: &str, data: &[u8]) -> Result<usize, FsError> {
    if name.len() == 0 || name.len() > MAX_NAME {
        return Err(FsError::NameTooLong);
    }
    if data.len() > MAX_FILE_SIZE {
        return Err(FsError::FileTooLarge);
    }

    let idx = match fs.find_by_name(name) {
        Some(i) => i,
        None => {
            let i = fs.find_free_slot().ok_or(FsError::NoSpace)?;
            let mut meta = FileMeta::empty();
            meta.used = true;
            meta.name_len = name.len() as u8;
            meta.len = 0;
            meta.slot = i as u16;
            meta.name[..name.len()].copy_from_slice(name.as_bytes());
            fs.metas[i] = meta;
            i
        }
    };

    let slot = fs.metas[idx].slot as usize;
    let base = FsState::slot_base(slot);
    let n = data.len();
    fs.storage[base..base + n].copy_from_slice(&data[..]);
    fs.metas[idx].len = n as u32;
    Ok(n)
}

// Convenience helpers for shell integration
pub fn cmd_ls() {
    fs_list(|name, len| {
        crate::vga::vprintln!("{:>6}  {}", len, name);
    });
}

pub fn cmd_cat(name: &str) {
    match fs_len(name) {
        Ok(len) => {
            let mut buf = [0u8; MAX_FILE_SIZE];
            let n = core::cmp::min(len, buf.len());
            match fs_read(name, &mut buf[..n]) {
                Ok(m) => {
                    let s = core::str::from_utf8(&buf[..m]).unwrap_or("<binary>");
                    crate::vga::vprintln!("{}", s);
                }
                Err(_) => crate::vga::vprintln!("cat: read error"),
            }
        }
        Err(_) => crate::vga::vprintln!("cat: not found"),
    }
}

pub fn cmd_write(name: &str, text: &str) {
    match fs_write(name, text.as_bytes()) {
        Ok(n) => crate::vga::vprintln!("wrote {} bytes to {}", n, name),
        Err(FsError::NoSpace) => crate::vga::vprintln!("write: no space"),
        Err(FsError::NameTooLong) => crate::vga::vprintln!("write: name too long"),
        Err(FsError::FileTooLarge) => crate::vga::vprintln!("write: file too large"),
        Err(FsError::NotFound) => crate::vga::vprintln!("write: not found"), // not hit here
    }
}