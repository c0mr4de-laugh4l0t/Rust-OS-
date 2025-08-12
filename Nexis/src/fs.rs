#![no_std]

use core::str;
use core::sync::atomic::{AtomicBool, Ordering};
use lazy_static::lazy_static;
use spin::Mutex;

const MAX_FILES: usize = 64;
const MAX_NAME: usize = 32;
const FILE_CAP: usize = 4096;
const MAX_FDS: usize = 128;
const FD_BASE: usize = 3;

#[derive(Clone, Copy)]
struct File {
    used: bool,
    name: [u8; MAX_NAME],
    len: usize,
    data: [u8; FILE_CAP],
}

impl File {
    const fn empty() -> Self {
        Self {
            used: false,
            name: [0u8; MAX_NAME],
            len: 0,
            data: [0u8; FILE_CAP],
        }
    }
}

#[derive(Clone, Copy)]
struct FdEntry {
    used: bool,
    file_idx: usize,
    pos: usize,
}

impl FdEntry {
    const fn empty() -> Self {
        Self { used: false, file_idx: 0, pos: 0 }
    }
}

lazy_static! {
    static ref FILE_TABLE: Mutex<[File; MAX_FILES]> = Mutex::new([File::empty(); MAX_FILES]);
    static ref FD_TABLE: Mutex<[FdEntry; MAX_FDS]> = Mutex::new([FdEntry::empty(); MAX_FDS]);
    static ref INIT_FLAG: AtomicBool = AtomicBool::new(false);
}

pub fn fs_init() {
    if INIT_FLAG.load(Ordering::SeqCst) { return; }
    INIT_FLAG.store(true, Ordering::SeqCst);
    let mut ft = FILE_TABLE.lock();
    ft[0].used = true;
    let name = b"README.txt";
    ft[0].name[..name.len()].copy_from_slice(name);
    let content = b"IronVeil RAMFS\n";
    ft[0].data[..content.len()].copy_from_slice(content);
    ft[0].len = content.len();
}

fn find_free_file_slot() -> Option<usize> {
    let mut ft = FILE_TABLE.lock();
    for i in 0..MAX_FILES {
        if !ft[i].used { return Some(i); }
    }
    None
}

fn find_file_by_name(name: &[u8]) -> Option<usize> {
    let ft = FILE_TABLE.lock();
    'outer: for i in 0..MAX_FILES {
        if !ft[i].used { continue; }
        let mut j = 0usize;
        while j < MAX_NAME {
            let c = ft[i].name[j];
            if c == 0 { break; }
            if j >= name.len() { continue 'outer; }
            if c != name[j] { continue 'outer; }
            j += 1;
        }
        if j == name.len() {
            return Some(i);
        }
    }
    None
}

fn alloc_fd() -> Option<usize> {
    let mut fdt = FD_TABLE.lock();
    for i in 0..MAX_FDS {
        if !fdt[i].used {
            fdt[i].used = true;
            fdt[i].file_idx = 0;
            fdt[i].pos = 0;
            return Some(i + FD_BASE);
        }
    }
    None
}

fn fd_to_index(fd: usize) -> Option<usize> {
    if fd < FD_BASE { return None; }
    let idx = fd - FD_BASE;
    if idx >= MAX_FDS { return None; }
    Some(idx)
}

pub fn fs_create(name_ptr: *const u8, name_len: usize, data_ptr: *const u8, data_len: usize) -> usize {
    if name_ptr.is_null() { return usize::MAX; }
    let name = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
    if name_len == 0 || name_len >= MAX_NAME { return usize::MAX; }
    if find_file_by_name(name).is_some() { return usize::MAX; }
    let slot = match find_free_file_slot() { Some(s) => s, None => return usize::MAX };
    let mut ft = FILE_TABLE.lock();
    ft[slot].used = true;
    for i in 0..MAX_NAME { ft[slot].name[i] = 0; }
    ft[slot].name[..name_len].copy_from_slice(&name[..]);
    let take = core::cmp::min(data_len, FILE_CAP);
    if !data_ptr.is_null() && take > 0 {
        let data = unsafe { core::slice::from_raw_parts(data_ptr, take) };
        ft[slot].data[..take].copy_from_slice(&data[..]);
        ft[slot].len = take;
    } else {
        ft[slot].len = 0;
    }
    slot
}

pub fn fs_open(name_ptr: *const u8, name_len: usize) -> usize {
    if name_ptr.is_null() { return usize::MAX; }
    let name = unsafe { core::slice::from_raw_parts(name_ptr, name_len) };
    if let Some(idx) = find_file_by_name(name) {
        if let Some(fd) = alloc_fd() {
            let mut fdt = FD_TABLE.lock();
            fdt[fd - FD_BASE].file_idx = idx;
            fdt[fd - FD_BASE].pos = 0;
            return fd;
        }
    }
    usize::MAX
}

pub fn fs_read(fd: usize, buf_ptr: *mut u8, len: usize) -> usize {
    let idx = match fd_to_index(fd) { Some(i) => i, None => return usize::MAX };
    let mut fdt = FD_TABLE.lock();
    if !fdt[idx].used { return usize::MAX; }
    let file_idx = fdt[idx].file_idx;
    let mut ft = FILE_TABLE.lock();
    if !ft[file_idx].used { return usize::MAX; }
    let available = ft[file_idx].len.saturating_sub(fdt[idx].pos);
    if available == 0 { return 0; }
    let to_copy = core::cmp::min(available, len);
    if buf_ptr.is_null() { return usize::MAX; }
    unsafe {
        let dst = core::slice::from_raw_parts_mut(buf_ptr, to_copy);
        let src = &ft[file_idx].data[fdt[idx].pos .. fdt[idx].pos + to_copy];
        dst.copy_from_slice(src);
    }
    fdt[idx].pos += to_copy;
    to_copy
}

pub fn fs_write(fd: usize, buf_ptr: *const u8, len: usize) -> usize {
    let idx = match fd_to_index(fd) { Some(i) => i, None => return usize::MAX };
    let mut fdt = FD_TABLE.lock();
    if !fdt[idx].used { return usize::MAX; }
    let file_idx = fdt[idx].file_idx;
    let mut ft = FILE_TABLE.lock();
    if !ft[file_idx].used { return usize::MAX; }
    let space = FILE_CAP.saturating_sub(fdt[idx].pos);
    if space == 0 { return 0; }
    let to_copy = core::cmp::min(space, len);
    if buf_ptr.is_null() || to_copy == 0 { return 0; }
    unsafe {
        let src = core::slice::from_raw_parts(buf_ptr, to_copy);
        let dst = &mut ft[file_idx].data[fdt[idx].pos .. fdt[idx].pos + to_copy];
        dst.copy_from_slice(src);
    }
    fdt[idx].pos += to_copy;
    if fdt[idx].pos > ft[file_idx].len { ft[file_idx].len = fdt[idx].pos; }
    to_copy
}

pub fn fs_close(fd: usize) -> usize {
    let idx = match fd_to_index(fd) { Some(i) => i, None => return usize::MAX };
    let mut fdt = FD_TABLE.lock();
    if !fdt[idx].used { return usize::MAX; }
    fdt[idx].used = false;
    fdt[idx].file_idx = 0;
    fdt[idx].pos = 0;
    0
}