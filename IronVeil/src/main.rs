use std::io::{self, Write};
use std::thread::sleep;
use std::time::Duration;

fn main() {
    banner();
    loop {
        print!("\x1b[38;5;208mironveil>\x1b[0m ");
        io::stdout().flush().unwrap();
        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let cmd = input.trim();

        match cmd {
            "help" => {
                println!("\x1b[38;5;208mAvailable commands:\x1b[0m");
                println!("  help    - Show this help message");
                println!("  clear   - Clear the screen");
                println!("  exit    - Quit IronVeil CLI");
            }
            "clear" => {
                print!("\x1b[2J\x1b[H");
            }
            "exit" => {
                println!("\x1b[38;5;208mExiting IronVeil CLI...\x1b[0m");
                break;
            }
            "" => {}
            _ => {
                println!("\x1b[31mUnknown command: {}\x1b[0m", cmd);
            }
        }
    }
}

fn banner() {
    let banner_lines = [
        "██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     ",
        "██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     ",
        "██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     ",
        "██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     ",
        "██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗",
        "╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝",
    ];
    print!("\x1b[2J\x1b[H");
    for line in &banner_lines {
        for c in line.chars() {
            print!("\x1b[38;5;208m{}\x1b[0m", c);
            io::stdout().flush().unwrap();
            sleep(Duration::from_millis(2));
        }
        println!();
    }
    println!();
}
