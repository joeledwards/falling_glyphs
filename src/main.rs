use crossterm::{
    cursor::{Hide, MoveTo, Show},
    event::{self, Event, KeyCode, KeyModifiers},
    style::{Color, Print, SetForegroundColor},
    terminal::{self, Clear, ClearType, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};
use rand::prelude::*;
use std::io::{self, stdout, Write};
use std::time::Duration;

struct GlyphStream {
    x: u16,
    y: i16,
    speed: u16,
    glyphs: Vec<char>,
}

impl GlyphStream {
    fn new(width: u16, height: u16) -> Self {
        let mut stream = Self {
            x: 0, // Will be set by reset
            y: 0, // Will be set by reset
            speed: 0, // Will be set by reset
            glyphs: Vec::new(), // Will be set by reset
        };
        stream.reset(width, height);
        stream
    }

    fn reset(&mut self, width: u16, height: u16) {
        let mut rng = rand::thread_rng();
        self.x = rng.gen_range(0..width);
        self.y = rng.gen_range(-(height as i16)..0);
        self.speed = rng.gen_range(1..3);
        let length = rng.gen_range(5..height / 2);
        self.glyphs = (0..length)
            .map(|_| std::char::from_u32(rng.gen_range(0x30A0..0x30FF)).unwrap_or('?'))
            .collect();
    }

    fn fall(&mut self, width: u16, height: u16) {
        if self.y >= height as i16 {
            self.reset(width, height);
        } else {
            self.y += self.speed as i16;
        }
    }

    fn erase(&self, stdout: &mut io::Stdout, height: u16) -> io::Result<()> {
        // Erase the characters that the stream is moving away from.
        for i in 0..self.speed {
            let erase_y = self.y - (i as i16);
            if erase_y >= 0 && erase_y < height as i16 {
                stdout
                    .execute(MoveTo(self.x, erase_y as u16))?
                    .execute(Print(' '))?;
            }
        }
        Ok(())
    }

    fn draw(&self, stdout: &mut io::Stdout, height: u16) -> io::Result<()> {
        for (i, &ch) in self.glyphs.iter().enumerate() {
            let current_y = self.y + i as i16;
            if current_y >= 0 && current_y < height as i16 {
                let color = if i == self.glyphs.len() - 1 {
                    Color::White
                } else if i > self.glyphs.len() - 5 {
                    Color::Green
                } else {
                    Color::DarkGreen
                };

                stdout
                    .execute(MoveTo(self.x, current_y as u16))?
                    .execute(SetForegroundColor(color))?
                    .execute(Print(ch))?;
            }
        }
        Ok(())
    }
}

fn main() -> io::Result<()> {
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    stdout.execute(Hide)?;
    terminal::enable_raw_mode()?;
    stdout.execute(Clear(ClearType::All))?;

    let (width, height) = terminal::size()?;
    let mut streams: Vec<GlyphStream> = (0..width / 2).map(|_| GlyphStream::new(width, height)).collect();

    loop {
        if event::poll(Duration::from_millis(75))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Char('q')
                    || key_event.code == KeyCode::Esc
                    || (key_event.code == KeyCode::Char('c')
                        && key_event.modifiers == KeyModifiers::CONTROL)
                {
                    break;
                }
            }
        }

        for stream in &mut streams {
            stream.erase(&mut stdout, height)?;
            stream.fall(width, height);
            stream.draw(&mut stdout, height)?;
        }

        stdout.flush()?;
    }

    stdout.execute(Show)?;
    stdout.execute(LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}