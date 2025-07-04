use rand::rngs::ThreadRng;
use rand::Rng;
use std::collections::VecDeque;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Default)]
pub struct DebugInfo {
    pub density: f64,
    pub update_delay: u64,
    pub updates_per_sec: f64,
    pub glyphs_per_sec: f64,
    pub glyphs_per_update: usize,
    pub stacks_per_update: usize,
    pub min_glyph_delay: u128,
    pub max_glyph_delay: u128,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnsiColor {
    White,
    Green,
    DarkGreen,
}


#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Glyph {
    pub value: char,
    pub color: AnsiColor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cell {
    pub ch: char,
    pub color: AnsiColor,
}

#[derive(Clone)]
pub struct Viewport {
    width: u16,
    height: u16,
    grid: Vec<Option<Cell>>,
}

impl Viewport {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            grid: vec![None; (width * height) as usize],
        }
    }

    pub fn get(&self, x: u16, y: u16) -> Option<Cell> {
        if x < self.width && y < self.height {
            self.grid
                .get((y * self.width + x) as usize)
                .cloned()
                .flatten()
        } else {
            None
        }
    }

    pub fn set(&mut self, x: u16, y: u16, cell: Cell) {
        if x < self.width && y < self.height {
            self.grid[(y * self.width + x) as usize] = Some(cell);
        }
    }
}

pub enum Change {
    Update(u16, u16, char, AnsiColor), // x, y, char, color
    Remove(u16, u16),                 // x, y
}

pub fn diff_viewports(old: &Viewport, new: &Viewport) -> Vec<Change> {
    let mut changes = Vec::new();
    for y in 0..new.height {
        for x in 0..new.width {
            let old_cell = old.get(x, y);
            let new_cell = new.get(x, y);
            if old_cell != new_cell {
                match new_cell {
                    Some(cell) => changes.push(Change::Update(x, y, cell.ch, cell.color)),
                    None => changes.push(Change::Remove(x, y)),
                }
            }
        }
    }
    changes
}

pub struct GlyphStack {
    pub x: u16,
    pub min_y: i16,
    pub max_y: i16,
    pub stack: VecDeque<Glyph>,
    pub length: u16,
    pub last_update: Instant,
    pub update_interval: Duration,
}

fn random_glyph() -> char {
    let mut rng = ThreadRng::default();
    std::char::from_u32(rng.gen_range(0x30A0..0x30FF)).unwrap_or('?')
}

impl GlyphStack {
    pub fn new(x: u16, height: u16) -> Self {
        let mut rng = ThreadRng::default();
        let length = rng.gen_range(1..height as u16 * 3 / 4);
        let update_interval = Duration::from_millis(rng.gen_range(50..250));

        let mut stack = VecDeque::with_capacity(length as usize);
        stack.push_front(Glyph {
            value: random_glyph(),
            color: AnsiColor::White,
        });

        Self {
            x,
            min_y: 0,
            max_y: 0,
            stack,
            length,
            last_update: Instant::now(),
            update_interval,
        }
    }

    pub fn update(&mut self) {
        if self.last_update.elapsed() >= self.update_interval {
            self.last_update = Instant::now();

            // Push a new, white glyph onto the stack
            self.stack.push_front(Glyph {
                value: random_glyph(),
                color: AnsiColor::White,
            });

            // Set the prior leading glyph to light green
            if self.stack.len() > 1 {
                if let Some(glyph) = self.stack.get_mut(1) {
                    glyph.color = AnsiColor::Green;
                }
            }

            // If the internal stack is > length, pop the oldest from the stack
            if self.stack.len() > self.length as usize {
                self.stack.pop_back();
                self.min_y += 1;
            }

            // Find the middle of the stack, and update that glyph to dark green
            if self.stack.len() > 2 {
                let mid = self.stack.len() / 2;
                if let Some(glyph) = self.stack.get_mut(mid) {
                    if glyph.color == AnsiColor::Green {
                        glyph.color = AnsiColor::DarkGreen;
                    }
                }
            }

            // 5% chance to change a random glyph
            let mut rng = ThreadRng::default();
            if self.stack.len() > 1 && rng.gen_bool(0.05) {
                let index = rng.gen_range(0..self.stack.len());
                if let Some(glyph) = self.stack.get_mut(index) {
                    glyph.value = random_glyph();
                }
            }

            self.max_y += 1;
        }
    }
}

pub struct Game {
    width: u16,
    height: u16,
    stacks: Vec<GlyphStack>,
    current_view: Viewport,
    density: f64,
    pub debug: bool,
    pub debug_info: DebugInfo,
    last_update_time: Instant,
    update_counter: u32,
    glyph_counter: usize,
}

impl Game {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            stacks: Vec::new(),
            current_view: Viewport::new(width, height),
            density: 0.5,
            debug: false,
            debug_info: DebugInfo::default(),
            last_update_time: Instant::now(),
            update_counter: 0,
            glyph_counter: 0,
        }
    }

    pub fn toggle_debug(&mut self) {
        self.debug = !self.debug;
    }

    pub fn get_dimensions(&self) -> (u16, u16) {
        (self.width, self.height)
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.width = width;
        self.height = height;
        self.current_view = Viewport::new(width, height);
        self.stacks
            .retain(|s| s.x < width && s.min_y < height as i16);
    }

    pub fn increase_density(&mut self) {
        self.density = (self.density + 0.1).min(1.0);
    }

    pub fn decrease_density(&mut self) {
        self.density = (self.density - 0.1).max(0.1);
    }

    pub fn update_and_get_changes(&mut self) -> Vec<Change> {
        let mut rng = ThreadRng::default();
        let mut stacks_this_update = 0;
        let mut glyphs_this_update = 0;

        // Determine whether any new stacks should be spawned
        if rng.gen_bool(self.density) {
            let x = rng.gen_range(0..self.width / 2) * 2;
            self.stacks.push(GlyphStack::new(x, self.height));
            stacks_this_update += 1;
        }

        // Update glyph stacks
        for stack in &mut self.stacks {
            let before_len = stack.stack.len();
            stack.update();
            let after_len = stack.stack.len();
            if after_len > before_len {
                glyphs_this_update += 1;
            }
        }

        // If y_min is outside of the viewport, delete the stack
        self.stacks.retain(|s| s.min_y < self.height as i16);

        let mut next_view = Viewport::new(self.width, self.height);
        for stack in &self.stacks {
            for (i, glyph) in stack.stack.iter().enumerate() {
                let y = stack.max_y - i as i16;
                if y >= 0 && y < self.height as i16 {
                    let cell_to_add = Cell {
                        ch: glyph.value,
                        color: glyph.color,
                    };
                    next_view.set(stack.x, y as u16, cell_to_add);
                }
            }
        }

        let changes = diff_viewports(&self.current_view, &next_view);
        self.current_view = next_view;

        // Update debug info
        self.update_counter += 1;
        self.glyph_counter += glyphs_this_update;
        let elapsed = self.last_update_time.elapsed();
        if elapsed >= Duration::from_secs(1) {
            self.debug_info.updates_per_sec = self.update_counter as f64 / elapsed.as_secs_f64();
            self.debug_info.glyphs_per_sec = self.glyph_counter as f64 / elapsed.as_secs_f64();
            self.update_counter = 0;
            self.glyph_counter = 0;
            self.last_update_time = Instant::now();
        }
        self.debug_info.density = self.density;
        self.debug_info.glyphs_per_update = glyphs_this_update;
        self.debug_info.stacks_per_update = stacks_this_update;
        let delays: Vec<u128> = self
            .stacks
            .iter()
            .map(|s| s.update_interval.as_millis())
            .collect();
        self.debug_info.min_glyph_delay = delays.iter().min().cloned().unwrap_or(0);
        self.debug_info.max_glyph_delay = delays.iter().max().cloned().unwrap_or(0);

        changes
    }
}
