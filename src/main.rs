use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout, Write};
use std::time::Duration;

mod game;
use game::{AnsiColor, Change, Game, DebugInfo};

fn convert_color(ansi_color: AnsiColor) -> Color {
    match ansi_color {
        AnsiColor::White => Color::White,
        AnsiColor::Green => Color::Green,
        AnsiColor::DarkGreen => Color::DarkGreen,
    }
}

fn render_debug_info(stdout: &mut io::Stdout, debug_info: &DebugInfo, width: u16) -> io::Result<u16> {
    let metrics: Vec<(&str, String)> = vec![
        ("Density:", format!("{:.1}", debug_info.density)),
        ("Updates/sec:", format!("{:.2}", debug_info.updates_per_sec)),
        ("Glyphs/sec:", format!("{:.2}", debug_info.glyphs_per_sec)),
        ("Glyphs/update:", format!("{}", debug_info.glyphs_per_update)),
        ("Stacks/update:", format!("{}", debug_info.stacks_per_update)),
        (
            "Min stack update delay (ms):",
            format!("{}", debug_info.min_glyph_delay),
        ),
        (
            "Max stack update delay (ms):",
            format!("{}", debug_info.max_glyph_delay),
        ),
    ];

    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for (label, value) in metrics {
        let metric_str = format!("{} {:<6}", label, value);
        if current_line.is_empty() {
            current_line.push_str(&metric_str);
        } else if current_line.len() + 3 + metric_str.len() <= width as usize {
            current_line.push_str(" | ");
            current_line.push_str(&metric_str);
        } else {
            lines.push(current_line.clone());
            current_line = metric_str;
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }

    stdout.execute(SetForegroundColor(Color::White))?;
    for (i, line) in lines.iter().enumerate() {
        stdout
            .execute(MoveTo(0, i as u16))?
            .execute(Print(format!("{:<width$}", line, width = width as usize)))?;
    }

    let num_lines = lines.len() as u16;
    let underscore_line = "_".repeat(width as usize);
    stdout
        .execute(MoveTo(0, num_lines))?
        .execute(Print(underscore_line))?;

    Ok(num_lines + 1)
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;
    terminal::enable_raw_mode()?;
    stdout.execute(Clear(ClearType::All))?;

    let (width, height) = terminal::size()?;
    let mut game = Game::new(width, height);
    let mut last_debug_state = game.debug;
    let mut last_debug_lines = 0;

    loop {
        if event::poll(Duration::from_millis(75))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('d') => game.increase_density(),
                    KeyCode::Char('D') => game.decrease_density(),
                    KeyCode::Char('?') => game.toggle_debug(),
                    _ => {}
                }
            }
        }

        let (current_width, current_height) = game.get_dimensions();
        let (new_width, new_height) = terminal::size()?;
        if new_width != current_width || new_height != current_height {
            game.resize(new_width, new_height);
            stdout.execute(Clear(ClearType::All))?;
        }

        let mut y_offset = 0;
        if game.debug {
            y_offset = render_debug_info(&mut stdout, &game.debug_info, new_width)?;
        } else if last_debug_state {
            for i in 0..last_debug_lines {
                stdout
                    .execute(MoveTo(0, i))?
                    .execute(Print(" ".repeat(new_width as usize)))?;
            }
        }
        last_debug_state = game.debug;
        last_debug_lines = y_offset;

        let changes = game.update_and_get_changes();
        for change in changes {
            match change {
                Change::Update(x, y, ch, color) => {
                    if y + y_offset < new_height {
                        stdout
                            .execute(MoveTo(x, y + y_offset))?
                            .execute(SetForegroundColor(convert_color(color)))?
                            .execute(Print(ch))?;
                    }
                }
                Change::Remove(x, y) => {
                    if y + y_offset < new_height {
                        stdout.execute(MoveTo(x, y + y_offset))?.execute(Print(' '))?;
                    }
                }
            }
        }

        stdout.flush()?;
    }

    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    stdout.execute(Clear(ClearType::All))?;
    terminal::disable_raw_mode()?;
    Ok(())
}
