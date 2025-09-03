use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};
use spin::Mutex;

/// Page/frame size — 4 KiB
pub const FRAME_SIZE: usize = 4096;

/// A physical frame address (aligned to FRAME_SIZE).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct PhysFrame(pub usize);

impl PhysFrame {
    #[inline]
    pub fn start_address(&self) -> usize {
        self.0
    }
}

/// Physical Memory Manager using a bitmap provided by caller.
///
/// NOTE: Caller must ensure the bitmap does not overlap managed memory.
pub struct PhysicalMemoryManager {
    bitmap: *mut u8,
    bitmap_len: usize,
    base_frame: usize,
    total_frames: usize,
    free_frames: AtomicUsize,
    lock: Mutex<()>,
}

impl PhysicalMemoryManager {
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

    pub fn init(
        &mut self,
        bitmap_ptr: *mut u8,
        bitmap_len: usize,
        base_frame: usize,
        total_frames: usize,
    ) {
        assert!(!bitmap_ptr.is_null(), "bitmap_ptr must not be null");
        let needed = (total_frames + 7) / 8;
        assert!(bitmap_len >= needed, "bitmap_len too small");

        unsafe {
            ptr::write_bytes(bitmap_ptr, 0, needed); // clear all bits (all free)
        }

        self.bitmap = bitmap_ptr;
        self.bitmap_len = bitmap_len;
        self.base_frame = base_frame;
        self.total_frames = total_frames;
        self.free_frames.store(total_frames, Ordering::SeqCst);
    }

    #[inline]
    pub fn total_frames(&self) -> usize {
        self.total_frames
    }

    #[inline]
    pub fn free_frames(&self) -> usize {
        self.free_frames.load(Ordering::SeqCst)
    }

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
                return false;
            }
            ptr::write_volatile(p, old | mask);
        }
        self.free_frames.fetch_sub(1, Ordering::SeqCst);
        true
    }

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
                return false;
            }
            ptr::write_volatile(p, old & !mask);
        }
        self.free_frames.fetch_add(1, Ordering::SeqCst);
        true
    }

    pub fn alloc_frame(&self) -> Option<PhysFrame> {
        let _g = self.lock.lock();
        let bytes = (self.total_frames + 7) / 8;

        for b in 0..bytes {
            let byte_val = unsafe { ptr::read_volatile(self.bitmap.add(b)) };
            if byte_val != 0xFF {
                for bit in 0..8 {
                    let idx = b * 8 + bit;
                    if idx >= self.total_frames {
                        break;
                    }
                    let mask = 1u8 << bit;
                    let cur = unsafe { ptr::read_volatile(self.bitmap.add(b)) };
                    if (cur & mask) == 0 {
                        unsafe { ptr::write_volatile(self.bitmap.add(b), cur | mask) };
                        self.free_frames.fetch_sub(1, Ordering::SeqCst);
                        return Some(PhysFrame((self.base_frame + idx) * FRAME_SIZE));
                    }
                }
            }
        }
        None
    }

    #[inline]
    pub fn free_frame(&self, addr: usize) -> bool {
        self.mark_free(addr)
    }

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
