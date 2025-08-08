// memory.rs
#![no_std]

use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Page/frame size — 4 KiB
pub const FRAME_SIZE: usize = 4096;

/// A physical frame address (physical addr aligned to FRAME_SIZE).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PhysFrame(pub usize);

impl PhysFrame {
    #[inline]
    pub fn start_address(&self) -> usize {
        self.0
    }
}

/// Physical Memory Manager using a bitmap provided by caller.
/// The bitmap is a contiguous region of bytes (each bit == one frame).
///
/// IMPORTANT: the caller MUST ensure the bitmap storage region does not overlap
/// the frames managed by the manager.
pub struct PhysicalMemoryManager {
    /// Pointer to bitmap storage (raw pointer, not owned)
    bitmap: *mut u8,
    /// bitmap length in bytes
    bitmap_len: usize,
    /// physical frame number of the first managed frame
    base_frame: usize,
    /// number of frames managed
    total_frames: usize,
    /// cached number of free frames (approx; updated atomically)
    free_frames: AtomicUsize,
    /// Protects bitmap operations
    lock: Mutex<()>,
}

impl PhysicalMemoryManager {
    /// Create an uninitialized (empty) manager.
    pub const fn new_uninit() -> Self {
        Self {
            bitmap: core::ptr::null_mut(),
            bitmap_len: 0,
            base_frame: 0,
            total_frames: 0,
            free_frames: AtomicUsize::new(0),
            lock: Mutex::new(()),
        }
    }

    /// Initialize the manager.
    ///
    /// - `bitmap_ptr` points to `bitmap_len` bytes available to store the bitmap.
    /// - `bitmap_len` is the length in bytes.
    /// - `base_frame` is the frame number (phys_addr / FRAME_SIZE) of first managed frame.
    /// - `total_frames` is how many frames are managed.
    ///
    /// The bitmap must be at least `(total_frames + 7) / 8` bytes long.
    pub fn init(
        &mut self,
        bitmap_ptr: *mut u8,
        bitmap_len: usize,
        base_frame: usize,
        total_frames: usize,
    ) {
        assert!(!bitmap_ptr.is_null(), "bitmap_ptr must not be null");
        let needed = (total_frames + 7) / 8;
        assert!(
            bitmap_len >= needed,
            "bitmap_len too small: need {} bytes, got {}",
            needed,
            bitmap_len
        );

        // Zero the bitmap initially (mark all frames free)
        unsafe {
            ptr::write_bytes(bitmap_ptr, 0, needed);
        }

        self.bitmap = bitmap_ptr;
        self.bitmap_len = bitmap_len;
        self.base_frame = base_frame;
        self.total_frames = total_frames;
        self.free_frames.store(total_frames, Ordering::SeqCst);
    }

    /// Return total frames managed
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    /// Return how many frames are free (approximate)
    pub fn free_frames(&self) -> usize {
        self.free_frames.load(Ordering::SeqCst)
    }

    /// Mark a physical frame as used (set bit)
    /// Returns `true` if the frame was previously free (now marked used),
    /// `false` otherwise (out of range or already used).
    pub fn mark_used(&self, phys_addr: usize) -> bool {
        let frame = phys_addr / FRAME_SIZE;
        if frame < self.base_frame || frame >= self.base_frame + self.total_frames {
            return false;
        }
        let idx = frame - self.base_frame;
        let byte = idx / 8;
        let bit = idx % 8;

        let _g = self.lock.lock();
        unsafe {
            let p = self.bitmap.add(byte);
            let old = ptr::read_volatile(p);
            let mask = 1u8 << bit;
            if (old & mask) != 0 {
                return false; // already set
            }
            ptr::write_volatile(p, old | mask);
        }
        self.free_frames.fetch_sub(1, Ordering::SeqCst);
        true
    }

    /// Mark a physical frame as free (clear bit)
    pub fn mark_free(&self, phys_addr: usize) -> bool {
        let frame = phys_addr / FRAME_SIZE;
        if frame < self.base_frame || frame >= self.base_frame + self.total_frames {
            return false;
        }
        let idx = frame - self.base_frame;
        let byte = idx / 8;
        let bit = idx % 8;

        let _g = self.lock.lock();
        unsafe {
            let p = self.bitmap.add(byte);
            let old = ptr::read_volatile(p);
            let mask = 1u8 << bit;
            if (old & mask) == 0 {
                return false; // already free
            }
            ptr::write_volatile(p, old & !mask);
        }
        self.free_frames.fetch_add(1, Ordering::SeqCst);
        true
    }

    /// Allocate a free frame and return its physical address, or None if exhausted.
    pub fn alloc_frame(&self) -> Option<PhysFrame> {
        let _g = self.lock.lock();

        // linear scan for first zero bit
        let bytes = (self.total_frames + 7) / 8;
        for b in 0..bytes {
            let byte_val = unsafe { core::ptr::read_volatile(self.bitmap.add(b)) };
            if byte_val != 0xFF {
                // found a byte with at least one free bit
                for bit in 0..8 {
                    let idx = b * 8 + bit;
                    if idx >= self.total_frames {
                        break;
                    }
                    let mask = 1u8 << bit;
                    let cur = unsafe { core::ptr::read_volatile(self.bitmap.add(b)) };
                    if (cur & mask) == 0 {
                        // set it
                        unsafe { core::ptr::write_volatile(self.bitmap.add(b), cur | mask) };
                        self.free_frames.fetch_sub(1, Ordering::SeqCst);
                        let frame_num = self.base_frame + idx;
                        let addr = frame_num * FRAME_SIZE;
                        return Some(PhysFrame(addr));
                    }
                }
            }
        }
        None
    }

    /// Free a previously allocated frame by physical address.
    pub fn free_frame(&self, addr: usize) -> bool {
        self.mark_free(addr)
    }

    /// Check whether a given physical address is marked used
    pub fn is_used(&self, phys_addr: usize) -> bool {
        let frame = phys_addr / FRAME_SIZE;
        if frame < self.base_frame || frame >= self.base_frame + self.total_frames {
            return false;
        }
        let idx = frame - self.base_frame;
        let byte = idx / 8;
        let bit = idx % 8;
        let cur = unsafe { ptr::read_volatile(self.bitmap.add(byte)) };
        (cur & (1u8 << bit)) != 0
    }
}
