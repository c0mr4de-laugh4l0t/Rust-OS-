use std::io;
use rand::{distributions::Alphanumeric, Rng};
use crossterm::{
    ExecutableCommand,
    terminal::{enable_raw_mode, disable_raw_mode, Clear, ClearType},
    cursor,
    style::{Color, Print, ResetColor, SetForegroundColor},
};

fn main() -> crossterm::Result<()> {
    let mut stdout = io::stdout();

    enable_raw_mode()?; 
    stdout.execute(Clear(ClearType::All))?; 
    stdout.execute(cursor::MoveTo(0, 0))?; 

    let banner = r#"
██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     
██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     
██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     
██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     
██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗
╚═╝╚═╝  ╚═╝ ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝
"#;

    stdout.execute(SetForegroundColor(Color::AnsiValue(208)))?;
    stdout.execute(Print(banner))?;
    stdout.execute(ResetColor)?;

    disable_raw_mode()?; 

    Ok(())
      }
