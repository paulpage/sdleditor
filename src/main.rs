extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep_ms;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{TextureQuery, WindowCanvas};
use sdl2::ttf::Font;

enum PaneType {
    Buffer,
    FileManager,
}

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

struct Buffer {
    contents: Vec<String>,
    name: String,
    is_dirty: bool,
}

struct Pane<'a> {
    pane_type: PaneType,
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    cursor_x: usize,
    // Allow the cursor to be restored to its original x coordinate
    // if it moves over a line with fewer characters
    max_cursor_x: usize,
    cursor_y: usize,
    font: &'a Font<'a, 'a>,
    line_height: i32,
    buffer_id: Option<usize>,
    scroll_idx: usize,
    // distance from top of document to top of viewport in pixels. Allows smooth scrolling.
    scroll_offset: i32,
}

impl<'a> Pane<'a> {
    fn fill_rect(&mut self, canvas: &mut WindowCanvas, color: Color, rect: Rect) {
        canvas.set_draw_color(color);
        let x = self.x + max(rect.x, 0);
        let y = self.y + max(rect.y, 0);
        let w = min(self.w as i32 - rect.x, rect.w);
        let h = min(self.h as i32 - rect.y, rect.h);
        if w > 0 && h > 0 {
            canvas.fill_rect(rect!(x, y, w, h)).unwrap();
        }
    }

    // Draw the given text on the given canvas with the given color.
    // Upper left corner of the text will be the x, y of the rect,
    // and text outside the width and height of the rect will be cut off.
    fn draw_text(&mut self, canvas: &mut WindowCanvas, color: Color, x: i32, y: i32, text: &str) {
        if y > 0 && x > 0 && !text.is_empty() {
            let mut left_bound = 0;
            let mut right_bound = text.len();

            let (width, _) = self.font.size_of(text).unwrap();

            // If the text we want to render is wider than the screen, do a binary
            // search to find the largest portion of the string that will fit on
            // the screen so we can just render that.
            if width > self.w {
                let mut i = 0;
                let mut seek = right_bound / 2;
                while (left_bound as i32 - right_bound as i32).abs() > 2 {
                    i += 1;
                    let (width, _) = self.font.size_of(&text[left_bound..seek]).unwrap();
                    if width > self.w {
                        right_bound = seek;
                        seek = left_bound + (right_bound - left_bound) / 2;
                    } else {
                        left_bound = seek;
                        seek = left_bound + (right_bound - left_bound) / 2;
                    }
                }
                let (width, _) = self.font.size_of(&text[0..seek + 1]).unwrap();
                right_bound = seek + 1;
            }

            let surface = self
                .font
                .render(&text[0..right_bound])
                .blended(color)
                .unwrap();
            let texture_creator = canvas.texture_creator();
            let texture = texture_creator
                .create_texture_from_surface(&surface)
                .unwrap();
            let TextureQuery {
                width: w,
                height: h,
                ..
            } = texture.query();
            let w = min(self.w as i32 - x, w as i32);
            let h = min(self.h as i32 - y, h as i32);
            let source = rect!(0, 0, w, h);
            let target = rect!(self.x + x, self.y + y, w, h);
            canvas.copy(&texture, Some(source), Some(target)).unwrap();
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
                let bar_height: i32 = (self.line_height + 5 * 2) as i32;
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
                    let (cx, _) = self
                        .font
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

struct App<'a> {
    buffers: Vec<Buffer>,
    panes: Vec<Pane<'a>>,
    buffer_idx: usize,
    pane_idx: usize,
}

fn main() -> Result<(), String> {
    // Initialize video subsystem
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Initialize app
    let mut app = App {
        buffers: Vec::new(),
        panes: Vec::new(),
        buffer_idx: 0,
        pane_idx: 0,
        // canvas: canvas,
    };

    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            app.buffers.push(Buffer {
                contents: Vec::new(),
                name: args[1].clone(),
                is_dirty: false,
            });
            let file = fs::File::open(&args[1]).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                app.buffers[app.buffer_idx].contents.push(line.unwrap());
            }
        }
        _ => {
            app.buffers.push(Buffer {
                contents: Vec::new(),
                name: "UNNAMED".to_string(),
                is_dirty: false,
            });
            app.buffers[app.buffer_idx].contents.push(String::new());
        }
    }

    let font = ttf_context
        .load_font("data/LiberationSans-Regular.ttf", 16)
        .unwrap();
    let (width, height) = canvas.window().size();
    app.panes.push(Pane {
        pane_type: PaneType::Buffer,
        x: 50,
        y: 50,
        w: width - 100,
        h: height - 100,
        cursor_x: 0,
        max_cursor_x: 0,
        cursor_y: 0,
        font: &font,
        line_height: 0,
        buffer_id: Some(0),
        scroll_idx: 0,
        scroll_offset: 0,
    });
    app.panes[app.pane_idx].line_height = app.panes[app.pane_idx].font.height();

    bool dirty = true;

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                _ => {
                    let pane = &mut app.panes[app.pane_idx];
                    match pane.pane_type {
                        PaneType::Buffer => {
                            if let Some(buffer) = pane.buffer_id {
                                let buffer = &mut app.buffers[buffer];
                                pane.handle_buffer_event(buffer, event);
                            }
                        }
                        PaneType::FileManager => {}
                    }
                }
            }
        }

        canvas.set_draw_color(Color::RGBA(200, 200, 200, 255));
        canvas.clear();

        for mut pane in &mut app.panes {
            // Smooth scrolling
            let target_scroll_offset = pane.scroll_idx as i32 * pane.line_height as i32;
            let scroll_delta = target_scroll_offset - pane.scroll_offset;
            if pane.scroll_offset < target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).ceil() as i32;
            } else if pane.scroll_offset > target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).floor() as i32;
            }

            if let Some(b) = pane.buffer_id {
                let buffer = &app.buffers[b];
                canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
                let rect = rect!(pane.x, pane.y, pane.w, pane.h);
                canvas.fill_rect(rect).unwrap();

                // We only want to render the lines that are actually on the screen.
                let first_line = max(
                    0,
                    pane.scroll_idx as i32
                        - (f64::from(height) / f64::from(pane.line_height)).ceil() as i32,
                ) as usize;
                let last_line = min(
                    buffer.contents.len(),
                    pane.scroll_idx
                        + (f64::from(height) / f64::from(pane.line_height)).ceil() as usize,
                );

                let padding: i32 = 5;
                let bar_height: i32 = (pane.line_height + padding * 2) as i32;
                // Draw the contents of the file and the cursor.
                for (i, entry) in buffer.contents[first_line..last_line].iter().enumerate() {
                    // Right-pad the string to allow the cursor to be rendered off the end of
                    // the line of text
                    let mut render_text = entry.clone();
                    if render_text.len() < pane.cursor_x {
                        for _ in render_text.len()..pane.cursor_x {
                            render_text.push(' ');
                        }
                    }

                    // Render the full line of text
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding,
                        bar_height + padding + (i as i32 + first_line as i32) * pane.line_height
                            - pane.scroll_offset,
                        &render_text,
                    );

                    // Draw the cursor if we're rendering the cursor line
                    if i == pane.cursor_y {
                        // If the cursor isn't at the beginning of the line, render the text
                        // before the cursor so we can measure its width.
                        let text_right = &render_text[..pane.cursor_x];
                        let (x, _) = pane.font.size_of(text_right).unwrap();
                        let rect = rect!(
                            padding + x as i32,
                            bar_height
                                + padding
                                + (i as i32 + first_line as i32) * pane.line_height
                                - pane.scroll_offset,
                            3,
                            pane.line_height
                        );
                        pane.fill_rect(&mut canvas, Color::RGBA(0, 0, 0, 255), rect);
                    }
                }

                // Draw bar
                let rect = rect!(0, 0, pane.w, bar_height);
                pane.fill_rect(&mut canvas, Color::RGBA(50, 50, 50, 255), rect);
                let dirty_text = if buffer.is_dirty { "*" } else { "" };
                let bar_text = format!("{} {}", dirty_text, &buffer.name);
                pane.draw_text(
                    &mut canvas,
                    Color::RGBA(200, 200, 200, 255),
                    padding,
                    padding,
                    &bar_text,
                );
            }
        }

        sleep_ms(15);
        canvas.present()
    }

    Ok(())
}
