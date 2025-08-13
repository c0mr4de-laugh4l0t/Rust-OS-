#![no_std]
#![no_main]

use bootloader::{entry_point, BootInfo};
use core::panic::PanicInfo;

mod interrupts;
mod vga;
mod kb;
mod memory;
mod scheduler;
mod syscall;
mod fs;

use vga::VgaWriter;
use crate::vga::VGA_WRITER;
use crate::kb::Kb;
use memory::{PhysicalMemoryManager, FRAME_SIZE};

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
    }

    unsafe {
        pmm_setup_linker();
    }

  unsafe { crate::alloc::init_heap(); }
  crate::fs::fs_init();

    fs::fs_init(); // new FS init here

    Kb::init();

    extern "C" fn shell_task() {
        shell_loop();
    }

    unsafe {
        crate::scheduler::spawn(shell_task, &PMM, 16);
    }

    crate::scheduler::schedule_loop()
}

fn shell_loop() -> ! {
    use kb::Kb;

    loop {
        crate::vga::vprint!("ironveil@nexis:~$ ");

        let line = Kb::read_line_irq();
        let cmd = line.trim();

        match cmd {
            "help" => {
                crate::vga::vprintln!("Commands: help, clear, ls, cat <file>, echo <msg> > <file>");
            }
            "clear" | "cls" => {
                VGA_WRITER.lock().clear_screen();
            }
            cmd if cmd.starts_with("ls") => {
                fs::list_files();
            }
            cmd if cmd.starts_with("cat ") => {
                let name = cmd.strip_prefix("cat ").unwrap().trim();
                fs::read_file(name);
            }
            cmd if cmd.starts_with("echo ") && cmd.contains(">") => {
                let parts: Vec<&str> = cmd.splitn(2, ">").collect();
                let data = parts[0].strip_prefix("echo ").unwrap().trim();
                let file = parts[1].trim();
                fs::write_file(file, data.as_bytes());
            }
            "" => {}
            _ => crate::vga::vprintln!("Unknown command: '{}'", cmd),
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    crate::vga::vprintln!("PANIC: {}", info);
    loop {}
}

unsafe fn pmm_setup_linker() {
    extern "C" {
        static __kernel_start: u8;
        static __kernel_end: u8;
    }
    let _kstart = &__kernel_start as *const _ as usize;
    let _kend = &__kernel_end as *const _ as usize;
}