extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::collections::HashMap;
use std::time::Duration;
use std::rc::Rc;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::{Font};

struct Buffer {
    name: String,
    contents: Vec<String>,
    is_dirty: bool,
}

#[derive(Hash)]
struct FontCacheKey {
    c: char,
    color: Color,
}

impl PartialEq for FontCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.c == other.c && self.color == other.color
    }
}
impl Eq for FontCacheKey {}

struct Pane<'a> {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    buffer_id: Option<usize>,
    cursor_x: usize,
    max_cursor_x: usize,
    cursor_y: usize,
    scroll_idx: usize,
    scroll_offset: i32,
    line_height: i32,
    font: Font<'a, 'static>,
    font_cache: HashMap<FontCacheKey, Rc<Texture>>,
}

fn fill_rect(pane: &mut Pane, canvas: &mut WindowCanvas, color: Color, rect: Rect) {
    canvas.set_draw_color(color);
    let x = pane.x + max(rect.x, 0);
    let y = pane.y + max(rect.y, 0);
    let w = min(pane.w as i32 - rect.x, rect.w) as u32;
    let h = min(pane.h as i32 - rect.y, rect.h) as u32;
    if w > 0 && h > 0 {
        canvas.fill_rect(Rect::new(x, y, w, h as u32)).unwrap();
    }
}

impl<'a> Pane<'a> {

    fn draw(&self, canvas: &mut WindowCanvas) {
        let rect = Rect::new(self.x, self.y, self.w, self.h);
        canvas.fill_rect(rect).unwrap();
    }
    fn draw_text(&mut self, canvas: &mut WindowCanvas, color: Color, x: i32, y: i32, text: &str) {
        if y > 0 && x > 0 {
            let mut length: i32 = 0;
            for c in text.chars() {
                let key = FontCacheKey {c: c, color: color };
                let texture = self
                    .font_cache
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| {
                        let surface = self
                            .font
                            .render(&c.to_string())
                            .blended(color)
                            .unwrap();
                        let texture = canvas
                            .texture_creator()
                            .create_texture_from_surface(&surface)
                            .unwrap();
                        let resource = Rc::new(texture);
                        println!("MISS");
                        self.font_cache.insert(key, resource.clone());
                        resource
                    });

                let TextureQuery {
                    width: mut w,
                    height: mut h,
                    ..
                } = texture.query();
                w = min(self.w as i32 - (x as i32 + length as i32), w as i32) as u32;
                h = min(self.h as i32 - y as i32, h as i32) as u32;
                let source = Rect::new(0, 0, w as u32, h as u32);
                let target = Rect::new(self.x + x + length as i32, self.y + y, w as u32, h as u32);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();

                if length > self.w as i32 {
                    return;
                }
                length += w as i32;
            }
        }
    }

    fn handle_buffer_event(&mut self, buffer: &mut Buffer, event: Event) {
        match event {
            Event::TextInput { text, .. } => {
                buffer.contents[self.cursor_y].insert_str(self.cursor_x, &text);
                self.cursor_x += 1;
                self.max_cursor_x += 1;
                buffer.is_dirty = true;
            }
            Event::MouseButtonDown { x, y, .. } => {
                let bar_height: u32 = (self.line_height + 5 * 2) as u32;
                let padding = 5;
                let mut y_idx = ((f64::from(y)
                        - f64::from(self.y)
                        - f64::from(padding)
                        - f64::from(bar_height))
                    / f64::from(self.line_height))
                    .floor() as usize
                    + self.scroll_idx;
                y_idx = min(y_idx, buffer.contents.len() - 1);
                let max_x_idx = buffer.contents[y_idx].len();

                self.cursor_y = y_idx;
                // Measure the length of each substring of the line until we get one that's
                // bigger than the x position of the mouse
                let mut x_idx = 0;
                let mut char_x = self.x + padding;
                let mut last_char_x = char_x;
                while char_x < x && (x_idx as usize) < max_x_idx + 1 {
                    let (cx, _) = self.font
                        .size_of(&buffer.contents[self.cursor_y][..x_idx])
                        .unwrap();
                    last_char_x = char_x;
                    char_x = self.x + padding + cx as i32;
                    x_idx += 1;
                }
                // If the mouse is on the right side of the character it's hovering over,
                // put the cursor on the right
                if (last_char_x as i32 - x as i32).abs() < (char_x as i32 - x as i32).abs() {
                    x_idx -= 1;
                }
                self.cursor_x = max(x_idx as i32 - 1, 0) as usize;
                self.max_cursor_x = self.cursor_x;
            }
            Event::MouseWheel { y, .. } => {
                let candidate = self.scroll_idx as i32 - (y * 3);
                if candidate < 0 {
                    self.scroll_idx = 0;
                } else if candidate > buffer.contents.len() as i32 {
                    self.scroll_idx = buffer.contents.len();
                } else {
                    self.scroll_idx = candidate as usize;
                }
            }
            Event::KeyDown {
                keycode: Some(kc),
                keymod,
                ..
            } => {
                match kc {
                    Keycode::Up => {
                        if self.cursor_y > 0 {
                            self.cursor_y -= 1;
                            self.cursor_x = max(
                                min(self.cursor_x, buffer.contents[self.cursor_y].len()),
                                min(self.max_cursor_x, buffer.contents[self.cursor_y].len()),
                            );
                        }
                    }
                    Keycode::Down => {
                        if self.cursor_y < buffer.contents.len() {
                            self.cursor_y += 1;
                            self.cursor_x = max(
                                min(self.cursor_x, buffer.contents[self.cursor_y].len()),
                                min(self.max_cursor_x, buffer.contents[self.cursor_y].len()),
                            );
                        }
                    }
                    Keycode::Left => {
                        if self.cursor_x > 0 {
                            self.cursor_x -= 1;
                            self.max_cursor_x = self.cursor_x;
                        }
                    }
                    Keycode::Right => {
                        if self.cursor_x < buffer.contents[self.cursor_y].len() {
                            self.cursor_x += 1;
                            self.max_cursor_x = self.cursor_x;
                        }
                    }
                    Keycode::PageUp => {
                        if self.scroll_idx < 3 {
                            self.scroll_idx = 0;
                            self.scroll_offset = 0;
                        } else {
                            self.scroll_idx -= 3;
                        }
                    }
                    Keycode::PageDown => {
                        if self.scroll_idx > buffer.contents.len() - 3 {
                            self.scroll_idx = buffer.contents.len();
                        } else {
                            self.scroll_idx += 3;
                        }
                    }
                    Keycode::Return => {
                        self.cursor_y += 1;
                        self.cursor_x = 0;
                        self.max_cursor_x = self.cursor_x;
                        buffer.contents.insert(self.cursor_y, String::new());
                    }
                    Keycode::Backspace => {
                        if self.cursor_x > 0 {
                            if self.cursor_x <= buffer.contents[self.cursor_y].len() {
                                buffer.contents[self.cursor_y].remove(self.cursor_x - 1);
                            }
                            self.cursor_x -= 1;
                            self.max_cursor_x = self.cursor_x;
                            buffer.is_dirty = true;
                        }
                    }
                    _ => {
                        if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                            match kc {
                                Keycode::Q => {
                                    // break 'mainloop;
                                }
                                Keycode::S => {
                                    println!("TODO: Save file");
                                }
                                Keycode::O => {
                                    println!("TODO: Open file");
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }

}

fn main() {
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();
    let mut buffers: Vec<Buffer> = Vec::new();
    let mut panes: Vec<Pane> = Vec::new();
    let mut buffer_idx = 0;
    let mut pane_idx = 0;

    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            buffers.push(Buffer {
                contents: Vec::new(),
                name: args[1].clone(),
                is_dirty: false,
            });
            let file = fs::File::open(&args[1]).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                buffers[buffer_idx].contents.push(line.unwrap());
            }
        }
        _ => {
            buffers.push(Buffer {
                contents: Vec::new(),
                name: "UNNAMED".to_string(),
                is_dirty: false,
            });
            buffers[buffer_idx].contents.push(String::new());
        }
    }

    let (_width, height) = canvas.window().size();
    panes.push(Pane {
        x: 100,
        y: 100,
        w: 400,
        h: 400,
        buffer_id: Some(0),
        cursor_x: 0,
        max_cursor_x: 0,
        cursor_y: 0,
        scroll_idx: 0,
        scroll_offset: 0,
        line_height: 0,
        font: ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap(),
        font_cache: HashMap::new(),
    });
    panes[pane_idx].line_height = panes[pane_idx].font.height();
    // panes[pane_idx].line_height = font_manager.font.height();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                _ => {
                    if let Some(b) = panes[pane_idx].buffer_id {
                        panes[pane_idx].handle_buffer_event(&mut buffers[b], event)
                    }
                }
            }
        }

        canvas.set_draw_color(Color::RGBA(40, 0, 0, 255));
        canvas.clear();

        canvas.set_draw_color(Color::RGBA(100, 100, 30, 255));

        for mut pane in &mut panes {
            pane.draw(&mut canvas);
            let line_height = pane.line_height;
            // Smooth scrolling
            let target_scroll_offset = pane.scroll_idx as i32 * line_height as i32;
            let scroll_delta = target_scroll_offset - pane.scroll_offset;
            if pane.scroll_offset < target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).ceil() as i32;
            } else if pane.scroll_offset > target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).floor() as i32;
            }
            let scroll_offset = pane.scroll_offset;

            if let Some(b) = pane.buffer_id {
                let buffer = &buffers[b];
                canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
                let rect = Rect::new(pane.x, pane.y, pane.w, pane.h);
                canvas.fill_rect(rect).unwrap();

                // We only want to render the lines that are actually on the screen.
                let first_line = max(
                    0,
                    pane.scroll_idx as i32
                    - (f64::from(height) / f64::from(line_height)).ceil() as i32,
                ) as usize;
                let last_line = min(
                    buffer.contents.len(),
                    pane.scroll_idx
                    + (f64::from(height) / f64::from(line_height)).ceil() as usize,
                );

                let padding: u32 = 5;
                let bar_height: u32 = line_height as u32 + padding * 2;
                // Draw the contents of the file and the cursor.

                for (i, entry) in buffer.contents[first_line..last_line].iter().enumerate() {

                    // Render the full line of text
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding as i32,
                        bar_height as i32 + padding as i32 + (i as i32 + first_line as i32) * line_height as i32
                        - scroll_offset,
                        &entry,
                    );

                    // Draw the cursor if we're rendering the cursor line
                    if i == pane.cursor_y {
                        // If the cursor isn't at the beginning of the line, render the text
                        // before the cursor so we can measure its width.
                        let text_right = &entry[..pane.cursor_x];
                        let (x, _) = pane.font.size_of(text_right).unwrap().clone();
                        let rect = Rect::new(
                            padding as i32 + x as i32,
                            bar_height as i32
                            + padding as i32
                            + (i as i32 + first_line as i32) * line_height as i32
                            - scroll_offset as i32,
                            3,
                            line_height as u32
                        );
                        fill_rect(&mut pane, &mut canvas, Color::RGBA(0, 0, 0, 255), rect);
                    }

                    // Draw bar
                    let rect = Rect::new(0, 0, pane.w, bar_height);
                    fill_rect(&mut pane, &mut canvas, Color::RGBA(50, 50, 50, 255), rect);
                    let dirty_text = if buffer.is_dirty { "*" } else { "" };
                    let bar_text = format!("{} {}", dirty_text, &buffer.name);
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(200, 200, 200, 255),
                        padding as i32,
                        padding as i32,
                        &bar_text,
                    );
                }
            }
        }

        sleep(Duration::from_millis(5));
        canvas.present(); 
    }
}
