#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

use nexis::{interrupts, kb::Kb, vga::{self, VGA_WRITER}, memory::{self, PhysicalMemoryManager, FRAME_SIZE}, scheduler, fs};

entry_point!(kernel_main);

static mut PMM: PhysicalMemoryManager = PhysicalMemoryManager::new_uninit();

fn kernel_main(_boot_info: &'static BootInfo) -> ! {
    interrupts::init_idt();
    interrupts::remap_pic();
    interrupts::enable_interrupts();

    {
        let mut vw = VGA_WRITER.lock();
        vw.clear_screen();
        vw.write_str("\n=== IronVeil / Nexis Kernel ===\n");
        vw.write_str("IRQ keyboard + PMM + Scheduler + Syscalls + FS\n\n");
    }

    vga::sprintln!("\n=== Nexis serial output ===");
    vga::sprintln!("System initialized. Type 'help'.\n");

    unsafe { pmm_setup_linker(); }
    Kb::init();
    fs::fs_init();

    extern "C" fn demo_task() {
        let mut i: u64 = 0;
        loop {
            vga::vprintln!("demo_task tick {}", i);
            i = i.wrapping_add(1);
            for _ in 0..200_000 { core::hint::spin_loop(); }
            scheduler::task_yield();
        }
    }

    extern "C" fn shell_task() {
        shell_loop();
    }

    unsafe {
        scheduler::spawn(demo_task, &PMM, 4);
        scheduler::spawn(shell_task, &PMM, 16);
    }

    scheduler::schedule_loop()
}

fn shell_loop() -> ! {
    use kb::Kb;
    let mut rng = kb::XorShift64::new(0xabcdef123456789u64);

    loop {
        vga::vprint!("ironveil@nexis:~$ ");
        vga::sprint!("ironveil@nexis:~$ ");

        let line = Kb::read_line_irq();
        let cmd = line.trim();

        match cmd {
            "help" => {
                vga::vprintln!("Available commands:");
                vga::vprintln!("  help       - this message");
                vga::vprintln!("  clear|cls  - clear screen");
                vga::vprintln!("  genpass    - generate password");
                vga::vprintln!("  ip         - fake IPv4");
                vga::vprintln!("  mac        - fake MAC");
                vga::vprintln!("  reboot     - halt");
                vga::vprintln!("  fs ls      - list demo files");
                vga::vprintln!("  fs cat <f> - print file contents");
            }
            "clear" | "cls" => VGA_WRITER.lock().clear_screen(),
            "genpass" => {
                let mut pass = [0u8; 16];
                for i in 0..16 { pass[i] = rng.next_range_u8(33u8, 126u8); }
                let p = unsafe { core::str::from_utf8_unchecked(&pass) };
                vga::vprintln!("Generated password: {}", p);
            }
            "ip" => {
                let a = rng.next_range_u8(10, 250);
                let b = rng.next_range_u8(1, 254);
                let c = rng.next_range_u8(1, 254);
                let d = rng.next_range_u8(1, 254);
                vga::vprintln!("Fake IPv4: {}.{}.{}.{}", a, b, c, d);
            }
            "mac" => {
                let mut parts = [0u8; 6];
                for i in 0..6 { parts[i] = rng.next_u8(); }
                vga::vprintln!("Fake MAC: {:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
                    parts[0], parts[1], parts[2], parts[3], parts[4], parts[5]);
            }
            "reboot" => loop { core::hint::spin_loop(); },
            x if x.starts_with("fs ls") => {
                let mut buf = [0u8; 256];
                let n = fs::list_files_syscall(buf.as_mut_ptr(), buf.len());
                if let Ok(out) = core::str::from_utf8(&buf[..n]) {
                    vga::vprintln!("{}", out);
                }
            }
            x if x.starts_with("fs cat ") => {
                let parts: Vec<&str> = x.splitn(3, ' ').collect();
                if parts.len() == 3 {
                    let fname = parts[2];
                    let mut out = [0u8; 512];
                    let n = fs::read_file_syscall(fname.as_ptr(), fname.len(), out.as_mut_ptr());
                    if let Ok(s) = core::str::from_utf8(&out[..n]) {
                        vga::vprintln!("{}", s);
                    }
                }
            }
            "" => {}
            _ => vga::vprintln!("Unknown command: '{}'", cmd),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    vga::vprintln!("\n*** PANIC ***");
    if let Some(loc) = info.location() {
        vga::vprintln!("at {}:{}: {}", loc.file(), loc.line(), info);
    } else {
        vga::vprintln!("panic: {}", info);
    }
    loop { core::hint::spin_loop(); }
}

unsafe fn pmm_setup_linker() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }

    let kstart = &__kernel_start as *const _ as usize;
    let kend = &__kernel_end as *const _ as usize;

    let kernel_end_page = ((kend + FRAME_SIZE - 1) / FRAME_SIZE) * FRAME_SIZE;
    let pool_start = if kernel_end_page < 0x0010_0000 { 0x0010_0000 } else { kernel_end_page };
    let pool_size = 64 * 1024 * 1024;
    let frames = pool_size / FRAME_SIZE;

    PMM.init(pool_start as *mut u8, pool_size, pool_start / FRAME_SIZE, frames);
    vga::vprintln!("PMM init done: {} frames", frames);
}