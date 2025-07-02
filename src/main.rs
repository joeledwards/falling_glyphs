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
use game::{AnsiColor, Change, Game};

fn convert_color(ansi_color: AnsiColor) -> Color {
    match ansi_color {
        AnsiColor::White => Color::White,
        AnsiColor::Green => Color::Green,
        AnsiColor::DarkGreen => Color::DarkGreen,
    }
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;
    terminal::enable_raw_mode()?;
    stdout.execute(Clear(ClearType::All))?;

    let (width, height) = terminal::size()?;
    let mut game = Game::new(width, height);

    loop {
        if event::poll(Duration::from_millis(75))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('c') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('d') if key_event.modifiers == KeyModifiers::CONTROL => break,
                    KeyCode::Char('d') => game.increase_density(),
                    KeyCode::Char('D') => game.decrease_density(),
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
        for change in changes {
            match change {
                Change::Update(x, y, ch, color) => {
                    stdout
                        .execute(MoveTo(x, y))?
                        .execute(SetForegroundColor(convert_color(color)))?
                        .execute(Print(ch))?;
                }
                Change::Remove(x, y) => {
                    stdout.execute(MoveTo(x, y))?.execute(Print(' '))?;
                }
            }
        }

        stdout.flush()?;
    }

    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}