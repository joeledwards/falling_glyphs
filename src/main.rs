use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use std::io::{self, stdout, Write};
use std::time::{Duration, Instant};

mod game;
use game::{Change, DebugInfo, Game};

fn convert_color(ansi_color: game::AnsiColor) -> Color {
    match ansi_color {
        game::AnsiColor::White => Color::White,
        game::AnsiColor::Green => Color::Green,
        game::AnsiColor::DarkGreen => Color::DarkGreen,
    }
}

fn render_debug_info(
    stdout: &mut io::Stdout,
    debug_info: &DebugInfo,
    width: u16,
    perf_lines: &[String],
) -> io::Result<u16> {
    let mut lines = Vec::new();

    // Helper to create a visual bar
    let create_bar = |value: f64, max_bar_width: usize| {
        let bar_fill = (value * max_bar_width as f64).round() as usize;
        let bar_empty = max_bar_width - bar_fill;
        format!("{}{}", "█".repeat(bar_fill), "░".repeat(bar_empty))
    };

    // --- Configurable Settings with Bars ---
    let label_width = 22;
    let value_width = 6;
    let bar_padding = 2; // for "  " around the bar
    let bar_width = if width > (label_width + value_width + bar_padding) as u16 {
        (width as usize) - label_width - value_width - bar_padding
    } else {
        10 // a minimum width
    };

    // Add settings lines to a temporary vector to be rendered with colors
    let mut settings_lines = Vec::new();
    // Density
    let density_percent = (debug_info.density - 0.1) / 0.9;
    settings_lines.push((
        "Density:",
        create_bar(density_percent, bar_width),
        format!("{:.1}", debug_info.density),
        Color::Green,
    ));
    // Max Stack Height
    let height_percent = (debug_info.max_stack_height - 0.1) / 0.9;
    settings_lines.push((
        "Max Stack Height:",
        create_bar(height_percent, bar_width),
        format!("{:.1}", debug_info.max_stack_height),
        Color::Yellow,
    ));
    // Speed
    let speed_percent = (debug_info.speed as f64 - 1.0) / 49.0;
    settings_lines.push((
        "Speed Level:",
        create_bar(speed_percent, bar_width),
        debug_info.speed.to_string(),
        Color::Blue,
    ));

    // --- Render all lines ---
    stdout.execute(SetForegroundColor(Color::White))?;
    for (i, (label, bar, value, color)) in settings_lines.iter().enumerate() {
        stdout
            .execute(MoveTo(0, i as u16))?
            .execute(Print(format!("{:<label_width$}", label, label_width = label_width)))?
            .execute(SetForegroundColor(*color))?
            .execute(Print(&bar))?
            .execute(SetForegroundColor(Color::White))?
            .execute(Print(format!(" {:>value_width$}", value, value_width = value_width)))?;
    }
    lines.extend(vec!["".to_string(); settings_lines.len()]);

    lines.push("".to_string()); // Spacer line
    lines.extend_from_slice(perf_lines);

    // --- Render performance lines ---
    let base_y = settings_lines.len() as u16 + 1;
    for (i, line) in perf_lines.iter().enumerate() {
        stdout
            .execute(MoveTo(0, base_y + i as u16))?
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

    let mut last_perf_update = Instant::now();
    let mut cached_perf_lines: Vec<String> = Vec::new();

    loop {
        if event::poll(Duration::from_millis(75))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('d') => game.increase_density(),
                    KeyCode::Char('D') => game.decrease_density(),
                    KeyCode::Char('h') => game.increase_max_stack_height(),
                    KeyCode::Char('H') => game.decrease_max_stack_height(),
                    KeyCode::Char('s') => game.increase_speed(),
                    KeyCode::Char('S') => game.decrease_speed(),
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

        let changes = game.update_and_get_changes();

        let mut y_offset = 0;
        if game.debug {
            // Update performance metrics only once a second
            if last_perf_update.elapsed() >= Duration::from_secs(1) {
                let perf_metrics = vec![
                    format!("Updates/sec: {:.2}", game.debug_info.updates_per_sec),
                    format!("Glyphs/sec: {:.2}", game.debug_info.glyphs_per_sec),
                    format!("Glyphs/update: {}", game.debug_info.glyphs_per_update),
                    format!("Stacks/update: {}", game.debug_info.stacks_per_update),
                    format!(
                        "Min/Max stack update delay (ms): {}/{}",
                        game.debug_info.min_glyph_delay, game.debug_info.max_glyph_delay
                    ),
                ];

                let mut perf_line = String::new();
                cached_perf_lines.clear();
                for metric in perf_metrics {
                    if perf_line.is_empty() {
                        perf_line.push_str(&metric);
                    } else if perf_line.len() + 3 + metric.len() <= new_width as usize {
                        perf_line.push_str(" | ");
                        perf_line.push_str(&metric);
                    } else {
                        cached_perf_lines.push(perf_line);
                        perf_line = metric;
                    }
                }
                if !perf_line.is_empty() {
                    cached_perf_lines.push(perf_line);
                }
                last_perf_update = Instant::now();
            }

            y_offset = render_debug_info(&mut stdout, &game.debug_info, new_width, &cached_perf_lines)?;
        } else if last_debug_state {
            for i in 0..last_debug_lines {
                stdout
                    .execute(MoveTo(0, i))?
                    .execute(Print(" ".repeat(new_width as usize)))?;
            }
        }
        last_debug_state = game.debug;
        last_debug_lines = y_offset;

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
