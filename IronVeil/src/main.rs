use std::io::{self, Read};
use std::time::{Duration, Instant};

use chrono::Local;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use rand::{distributions::Alphanumeric, rngs::SmallRng, Rng, SeedableRng};
use tui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs, Wrap},
    Terminal,
};

#[cfg(feature = "kernel")]
mod os {
    #[inline]
    pub fn write(s: &str) {
        unsafe {
            core::arch::asm!(
                "mov rax, 0",
                "mov rdi, {ptr}",
                "mov rsi, {len}",
                "int 0x80",
                ptr = in(reg) s.as_ptr(),
                len = in(reg) s.len(),
                out("rax") _,
                options(nostack, preserves_flags)
            );
        }
    }
    #[allow(dead_code)]
    pub fn exit(code: i32) -> ! {
        unsafe {
            core::arch::asm!(
                "mov rax, 1",
                "mov rdi, {code}",
                "int 0x80",
                code = in(reg) code,
                options(noreturn)
            );
        }
    }
}

#[cfg(not(feature = "kernel"))]
mod os {
    #[inline]
    pub fn write(s: &str) {
        print!("{s}");
    }
    #[allow(dead_code)]
    pub fn exit(_code: i32) -> ! {
        std::process::exit(0);
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Tab {
    Dashboard,
    Security,
    Network,
    Files,
    Logs,
    About,
}
const ALL_TABS: [Tab; 6] = [
    Tab::Dashboard,
    Tab::Security,
    Tab::Network,
    Tab::Files,
    Tab::Logs,
    Tab::About,
];

impl Tab {
    fn title(self) -> &'static str {
        match self {
            Tab::Dashboard => "Dashboard",
            Tab::Security => "Security",
            Tab::Network => "Network",
            Tab::Files => "Files",
            Tab::Logs => "Logs",
            Tab::About => "About",
        }
    }
}

#[derive(Default)]
struct App {
    tab: Tab,
    sidebar_idx: usize,
    input: String,
    vpn_enabled: bool,
    logs: Vec<String>,
    files: Vec<String>,
    selected_file: Option<usize>,
    rng: SmallRng,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            tab: Tab::Dashboard,
            sidebar_idx: 0,
            input: String::new(),
            vpn_enabled: false,
            logs: vec![],
            files: vec![],
            selected_file: None,
            rng: SmallRng::from_entropy(),
        };
        app.refresh_files();
        app
    }

    fn log<S: Into<String>>(&mut self, s: S) {
        let s = s.into();
        let ts = Local::now().format("%H:%M:%S");
        let line = format!("[{}] {}\n", ts, &s);
        self.logs.push(line.trim_end().to_string());
        if self.logs.len() > 2000 {
            self.logs.drain(0..self.logs.len() - 2000);
        }
        os::write(&line);
    }

    fn refresh_files(&mut self) {
        self.files.clear();
        #[cfg(not(feature = "kernel"))]
        {
            if let Some(home) = dirs::home_dir() {
                if let Ok(read_dir) = std::fs::read_dir(home) {
                    for e in read_dir.flatten().take(200) {
                        if let Some(name) = e.file_name().to_str() {
                            self.files.push(name.to_string());
                        }
                    }
                }
            }
        }
        #[cfg(feature = "kernel")]
        {
            self.files.clear();
        }
    }

    fn gen_password(&mut self) -> String {
        let s: String = (&mut self.rng)
            .sample_iter(&Alphanumeric)
            .take(16)
            .map(char::from)
            .collect();
        self.log(format!("Generated password: {}", s));
        s
    }

    fn fake_ip(&mut self) -> String {
        let a = self.rng.gen_range(10..=250);
        let b = self.rng.gen_range(1..=254);
        let c = self.rng.gen_range(1..=254);
        let d = self.rng.gen_range(1..=254);
        let ip = format!("{}.{}.{}.{}", a, b, c, d);
        self.log(format!("Fake IPv4: {}", ip));
        ip
    }

    fn fake_mac(&mut self) -> String {
        let p: [u8; 6] = self.rng.gen();
        let mac = format!(
            "{:02x}:{:02x}:{:02x}:{:02x}:{:02x}:{:02x}",
            p[0], p[1], p[2], p[3], p[4], p[5]
        );
        self.log(format!("Fake MAC: {}", mac));
        mac
    }

    fn secure_delete(&mut self, name: &str) {
        self.log(format!("Secure-deleting '{}' (simulated)", name));
    }

    fn toggle_vpn(&mut self) {
        self.vpn_enabled = !self.vpn_enabled;
        self.log(if self.vpn_enabled { "VPN enabled" } else { "VPN disabled" });
    }

    fn run_command(&mut self, cmd: &str) {
        match cmd.trim() {
            "" => {}
            "help" => {
                self.log("Commands: help, genpass, ip, mac, vpn, sdel <name>, clear, exit");
            }
            "genpass" => {
                let _ = self.gen_password();
            }
            "ip" => {
                let _ = self.fake_ip();
            }
            "mac" => {
                let _ = self.fake_mac();
            }
            "vpn" => self.toggle_vpn(),
            x if x.starts_with("sdel ") => {
                let name = x.trim_start_matches("sdel ").trim();
                if name.is_empty() {
                    self.log("Usage: sdel <name>");
                } else {
                    self.secure_delete(name);
                }
            }
            "clear" => {
                self.logs.clear();
            }
            "exit" => {
                self.log("Exiting...");
                #[cfg(feature = "kernel")]
                {
                    os::exit(0);
                }
                #[cfg(not(feature = "kernel"))]
                {
                    std::process::exit(0);
                }
            }
            other => {
                self.log(format!("Unknown: '{}'", other));
            }
        }
    }
}

fn main() -> anyhow::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.clear()?;

    let mut app = App::new();
    let mut last_tick = Instant::now();
    let tick_rate = Duration::from_millis(100);

    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        if crossterm::event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if handle_key(&mut app, key)? {
                    break;
                }
            }
        }

        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
        }
    }

    disable_raw_mode()?;
    let mut stdout = terminal.into_inner();
    execute!(stdout, LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn handle_key(app: &mut App, key: KeyEvent) -> anyhow::Result<bool> {
    match (key.modifiers, key.code) {
        (KeyModifiers::CONTROL, KeyCode::Char('c')) => return Ok(true),
        (_, KeyCode::Esc) => return Ok(true),

        (_, KeyCode::Left) => {
            let idx = ALL_TABS.iter().position(|t| *t == app.tab).unwrap_or(0);
            let n = if idx == 0 { ALL_TABS.len() - 1 } else { idx - 1 };
            app.tab = ALL_TABS[n];
        }
        (_, KeyCode::Right) => {
            let idx = ALL_TABS.iter().position(|t| *t == app.tab).unwrap_or(0);
            let n = (idx + 1) % ALL_TABS.len();
            app.tab = ALL_TABS[n];
        }

        (_, KeyCode::Tab) => {
            app.sidebar_idx = app.sidebar_idx.saturating_add(1) % sidebar_items(app.tab).len();
        }
        (KeyModifiers::SHIFT, KeyCode::Tab) => {
            let len = sidebar_items(app.tab).len();
            app.sidebar_idx = (app.sidebar_idx + len - 1) % len;
        }

        (_, KeyCode::Enter) => {
            match app.tab {
                Tab::Security => match app.sidebar_idx {
                    0 => {
                        let _ = app.gen_password();
                    }
                    1 => app.secure_delete("secret.txt"),
                    2 => app.toggle_vpn(),
                    _ => {}
                },
                Tab::Network => match app.sidebar_idx {
                    0 => {
                        let _ = app.fake_ip();
                    }
                    1 => {
                        let _ = app.fake_mac();
                    }
                    _ => {}
                },
                Tab::Files => match app.sidebar_idx {
                    0 => app.refresh_files(),
                    1 => if let Some(sel) = app.selected_file {
                        let name = &app.files[sel];
                        app.log(format!("Open '{}': (preview below)", name));
                    },
                    _ => {}
                },
                Tab::Dashboard | Tab::Logs | Tab::About => {}
            }
        }

        (_, KeyCode::Char('j')) | (_, KeyCode::Down) => {
            if app.tab == Tab::Files {
                if let Some(sel) = app.selected_file {
                    if sel + 1 < app.files.len() {
                        app.selected_file = Some(sel + 1);
                    }
                } else if !app.files.is_empty() {
                    app.selected_file = Some(0);
                }
            }
        }
        (_, KeyCode::Char('k')) | (_, KeyCode::Up) => {
            if app.tab == Tab::Files {
                if let Some(sel) = app.selected_file {
                    if sel > 0 {
                        app.selected_file = Some(sel - 1);
                    }
                }
            }
        }

        (_, KeyCode::Backspace) => {
            app.input.pop();
        }
        (_, KeyCode::Char(c)) => {
            if c == '\n' {
                app.run_command(&app.input);
                app.input.clear();
            } else {
                app.input.push(c);
            }
        }

        _ => {}
    }
    Ok(false)
}

fn ui<B: tui::backend::Backend>(f: &mut tui::Frame<B>, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(3), Constraint::Min(1), Constraint::Length(3)].as_ref())
        .split(f.size());

    draw_banner(f, chunks[0]);
    draw_body(f, chunks[1], app);
    draw_cmdline(f, chunks[2], &app.input);
}

fn draw_banner<B: tui::backend::Backend>(f: &mut tui::Frame<B>, area: Rect) {
    let title = vec![Spans::from(Span::styled(
        "██╗██████╗  ██████╗ ███╗   ██╗██╗   ██╗███████╗██╗██╗     \n\
         ██║██╔══██╗██╔═══██╗████╗  ██║██║   ██║██╔════╝██║██║     \n\
         ██║██████╔╝██║   ██║██╔██╗ ██║██║   ██║█████╗  ██║██║     \n\
         ██║██╔══██╗██║   ██║██║╚██╗██║╚██╗ ██╔╝██╔══╝  ██║██║     \n\
         ██║██║  ██║╚██████╔╝██║ ╚████║ ╚████╔╝ ███████╗██║███████╗\n\
         ╚═╝╚═╝  ╚═════╝ ╚═╝  ╚═══╝  ╚═══╝  ╚══════╝╚═╝╚══════╝",
        Style::default().fg(Color::Indexed(208)).add_modifier(Modifier::BOLD),
    ))];
    let block = Paragraph::new(title).wrap(Wrap { trim: false });
    f.render_widget(block, area);
}

fn draw_body<B: tui::backend::Backend>(f: &mut tui::Frame<B>, area: Rect, app: &mut App) {
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Length(24), Constraint::Min(10)].as_ref())
        .split(area);

    draw_sidebar(f, body[0], app);
    draw_main(f, body[1], app);
}

fn draw_sidebar<B: tui::backend::Backend>(f: &mut tui::Frame<B>, area: Rect, app: &mut App) {
    let titles: Vec<Spans> = ALL_TABS
        .iter()
        .map(|t| Spans::from(Span::styled(t.title(), Style::default().fg(Color::Cyan))))
        .collect();

    let tabs = Tabs::new(titles)
        .select(ALL_TABS.iter().position(|t| *t == app.tab).unwrap_or(0))
        .block(Block::default().borders(Borders::ALL).title("Tabs"))
        .highlight_style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);

    let inner = Rect { x: area.x, y: area.y + 3, width: area.width, height: area.height.saturating_sub(3) };
    let items = sidebar_items(app.tab);
    let list_items: Vec<ListItem> = items.iter().enumerate().map(|(i, s)| {
        let st = if i == app.sidebar_idx { Style::default().fg(Color::Green).add_modifier(Modifier::BOLD) } else { Style::default() };
        ListItem::new(Span::styled(*s, st))
    }).collect();

    let block = Block::default().borders(Borders::ALL).title("Actions");
    f.render_widget(block, inner);

    let list_area = Rect { x: inner.x + 1, y: inner.y + 1, width: inner.width.saturating_sub(2), height: inner.height.saturating_sub(2) };
    f.render_widget(List::new(list_items), list_area);
}

fn sidebar_items(tab: Tab) -> &'static [&'static str] {
    match tab {
        Tab::Dashboard => &["Status refresh", "Show time"],
        Tab::Security => &["Generate password", "Secure delete", "Toggle VPN"],
        Tab::Network => &["Fake IPv4", "Fake MAC"],
        Tab::Files => &["Refresh list", "Open selected"],
        Tab::Logs => &["Clear logs"],
        Tab::About => &["Project site", "License"],
    }
}

fn draw_main<B: tui::backend::Backend>(f: &mut tui::Frame<B>, area: Rect, app: &mut App) {
    match app.tab {
        Tab::Dashboard => {
            let text = vec![
                Spans::from(Span::styled("IronVeil — User-space Shell", Style::default().add_modifier(Modifier::BOLD))),
                Spans::from(format!("Time: {}", Local::now().format("%Y-%m-%d %H:%M:%S"))),
                Spans::from(format!("VPN: {}", if app.vpn_enabled { "ENABLED" } else { "DISABLED" })),
                Spans::from(""),
                Spans::from("Use ←/→ to change tabs, Tab/Shift+Tab to change actions, Enter to run."),
            ];
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Dashboard"))
                .wrap(Wrap { trim: true });
            f.render_widget(p, area);
        }
        Tab::Security => {
            let text = vec![
                Spans::from("Security utilities:"),
                Spans::from("  • Generate strong password"),
                Spans::from("  • Secure delete (simulated)"),
                Spans::from("  • Toggle VPN state"),
            ];
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Security"))
                .wrap(Wrap { trim: true });
            f.render_widget(p, area);
        }
        Tab::Network => {
            let text = vec![
                Spans::from("Network tools:"),
                Spans::from("  • Fake IPv4"),
                Spans::from("  • Fake MAC"),
            ];
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("Network"))
                .wrap(Wrap { trim: true });
            f.render_widget(p, area);
        }
        Tab::Files => {
            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([Constraint::Percentage(50), Constraint::Percentage(50)].as_ref())
                .split(area);

            let list_items: Vec<ListItem> = app.files.iter().enumerate().map(|(i, name)| {
                let st = if Some(i) == app.selected_file {
                    Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)
                } else { Style::default() };
                ListItem::new(Span::styled(name.clone(), st))
            }).collect();

            let left = Block::default().borders(Borders::ALL).title("Home files");
            f.render_widget(left, chunks[0]);
            let list_area = Rect { x: chunks[0].x + 1, y: chunks[0].y + 1, width: chunks[0].width.saturating_sub(2), height: chunks[0].height.saturating_sub(2) };
            f.render_widget(List::new(list_items), list_area);

            let right = Block::default().borders(Borders::ALL).title("Preview");
            f.render_widget(right, chunks[1]);

            if let Some(sel) = app.selected_file {
                let name = &app.files[sel];
                #[cfg(not(feature = "kernel"))]
                if let Some(home) = dirs::home_dir() {
                    let path = home.join(name);
                    let mut preview = String::new();
                    if let Ok(mut f) = std::fs::File::open(path) {
                        let mut buf = vec![0u8; 8192];
                        if let Ok(n) = f.read(&mut buf) {
                            preview = String::from_utf8_lossy(&buf[..n]).to_string();
                        }
                    }
                    let para = Paragraph::new(preview).wrap(Wrap { trim: false });
                    let inner = Rect { x: chunks[1].x + 1, y: chunks[1].y + 1, width: chunks[1].width.saturating_sub(2), height: chunks[1].height.saturating_sub(2) };
                    f.render_widget(para, inner);
                }
            }
        }
        Tab::Logs => {
            let lines: Vec<Spans> = app.logs.iter().rev().take(300).map(|s| Spans::from(s.clone())).collect();
            let p = Paragraph::new(lines)
                .block(Block::default().borders(Borders::ALL).title("Logs"))
                .wrap(Wrap { trim: false });
            f.render_widget(p, area);
        }
        Tab::About => {
            let text = vec![
                Spans::from(Span::styled("IronVeil", Style::default().add_modifier(Modifier::BOLD))),
                Spans::from("User-space shell & tools (hosted)."),
                Spans::from("When built with feature `kernel`, logs also go through Nexis `int 0x80` (sys_write)."),
            ];
            let p = Paragraph::new(text)
                .block(Block::default().borders(Borders::ALL).title("About"))
                .wrap(Wrap { trim: true });
            f.render_widget(p, area);
        }
    }
}

fn draw_cmdline<B: tui::backend::Backend>(f: &mut tui::Frame<B>, area: Rect, input: &str) {
    let p = Paragraph::new(Spans::from(vec![
        Span::styled("ironveil> ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        Span::raw(input),
    ]))
    .block(Block::default().borders(Borders::ALL).title("Command"))
    .wrap(Wrap { trim: false });

    f.render_widget(p, area);
}