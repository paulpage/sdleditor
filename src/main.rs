extern crate sdl2;

use std::fs;
use std::io::{BufReader, BufRead};
use std::env;
use std::cmp::{min, max};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::rect::Rect;
use sdl2::render::{WindowCanvas, TextureQuery};
use sdl2::pixels::Color;
use sdl2::ttf::Font;

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
    x: i32 ,
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
            let surface = self.font.render(text).blended(color).unwrap();
            let texture_creator = canvas.texture_creator();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            let TextureQuery { width: w, height: h, .. } = texture.query();
            let w = min(self.w as i32 - x, w as i32);
            let h = min(self.h as i32 - y, h as i32);
            let source = rect!(0, 0, w, h);
            let target = rect!(self.x + x, self.y + y, w, h);
            canvas.copy(&texture, Some(source), Some(target)).unwrap();
        }
    }
}

struct App<'a> {
    line_height: i32,
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
    let window = video_subsys.window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    // Initialize app
    let mut app = App {
        line_height: 16,
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
        },
        _ => {
            app.buffers.push(Buffer {
                contents: Vec::new(),
                name: "UNNAMED".to_string(),
                is_dirty: false,
            });
            app.buffers[app.buffer_idx].contents.push(String::new());
        },
    }
    let font = ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap();
    let (width, height) = canvas.window().size();
    app.panes.push(Pane {
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
    // app.panes.push(Pane {
    //     x: 100,
    //     y: 100,
    //     w: width - 100,
    //     h: height - 100,
    //     cursor_x: 0,
    //     cursor_y: 0,
    //     font: &font,
    //     line_height: 0,
    //     buffer_id: Some(0),
    //     scroll_idx: 0,
    //     scroll_offset: 0,
    // });
    // app.panes[app.pane_idx + 1].line_height = app.panes[app.pane_idx].font.height();

    app.line_height = app.panes[app.pane_idx].font.height();

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::Quit {..} => break 'mainloop,
                Event::TextInput { text, .. } => {
                    app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].insert_str(app.panes[app.pane_idx].cursor_x, &text);
                    app.panes[app.pane_idx].cursor_x += 1;
                    app.panes[app.pane_idx].max_cursor_x += 1;
                    app.buffers[app.buffer_idx].is_dirty = true;
                },
                Event::MouseButtonDown { x, y, .. } => {
                    let pane = &mut app.panes[app.pane_idx];
                    let bar_height: i32 = (pane.line_height + 5 * 2) as i32;
                    let padding = 5;
                    let mut y_idx = ((f64::from(y) - f64::from(pane.y) - f64::from(padding) - f64::from(bar_height) / f64::from(pane.line_height)).floor()) as usize + pane.scroll_idx;
                    let mut max_x_idx = 0;
                    if let Some(b) = pane.buffer_id {
                        y_idx = min(y_idx, app.buffers[b].contents.len() - 1);
                        max_x_idx = app.buffers[b].contents[y_idx].len();
                    }

                    pane.cursor_y = y_idx;
                    // Measure the length of each substring of the line until we get one that's
                    // bigger than the x position of the mouse
                    let mut x_idx = 0;
                    let mut char_x = pane.x + padding;
                    let mut last_char_x = char_x;
                    // while char_x < x && (x_idx as usize) < max_x_idx {
                    while char_x < x && (x_idx as usize) < max_x_idx + 1 {
                        let (cx, _) = font.size_of(&app.buffers[app.buffer_idx].contents[pane.cursor_y][..x_idx]).unwrap();
                        last_char_x = char_x;
                        char_x = pane.x + padding + cx as i32;
                        x_idx += 1;
                    }
                    // If the mouse is on the right side of the character it's hovering over,
                    // put the cursor on the right
                    if (last_char_x as i32 - x as i32).abs() < (char_x as i32 - x as i32).abs() {
                        x_idx -= 1;
                    }
                    pane.cursor_x = max(x_idx as i32 - 1, 0) as usize;
                    pane.max_cursor_x = pane.cursor_x;
                },
                Event::MouseWheel { y, .. } => {
                    let candidate = app.panes[app.pane_idx].scroll_idx as i32 - (y * 3);
                    if candidate < 0 {
                        app.panes[app.pane_idx].scroll_idx = 0;
                    } else if candidate > app.buffers[app.buffer_idx].contents.len() as i32 {
                        app.panes[app.pane_idx].scroll_idx = app.buffers[app.buffer_idx].contents.len();
                    } else {
                        app.panes[app.pane_idx].scroll_idx = candidate as usize;
                    }
                },
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    let pane = &mut app.panes[app.pane_idx];
                    let buffer = match pane.buffer_id {
                        Some(b) => Some(&app.buffers[b]),
                        None => None,
                    };
                    match kc {
                        Keycode::Up => {
                            if let Some(buffer) = buffer {
                                if pane.cursor_y > 0 {
                                    pane.cursor_y -= 1;
                                    pane.cursor_x = max(
                                        min(pane.cursor_x, buffer.contents[pane.cursor_y].len()),
                                        min(pane.max_cursor_x, buffer.contents[pane.cursor_y].len()));
                                }
                            }

                        },
                        Keycode::Down => {
                            if let Some(buffer) = buffer {
                                if pane.cursor_y < buffer.contents.len() {
                                    pane.cursor_y += 1;
                                    pane.cursor_x = max(
                                        min(pane.cursor_x, buffer.contents[pane.cursor_y].len()),
                                        min(pane.max_cursor_x, buffer.contents[pane.cursor_y].len()));
                                }
                            }
                        },
                        Keycode::Left => {
                            if pane.cursor_x > 0 {
                                pane.cursor_x -= 1;
                                pane.max_cursor_x = pane.cursor_x;
                            }
                        },
                        Keycode::Right => {
                            if let Some(buffer) = buffer {
                                if pane.cursor_x < buffer.contents[pane.cursor_y].len() {
                                    pane.cursor_x += 1;
                                    pane.max_cursor_x = pane.cursor_x;
                                }
                            }
                        }
                        Keycode::PageUp => {
                            if pane.scroll_idx < 3 {
                                pane.scroll_idx = 0;
                                pane.scroll_offset = 0;
                            } else {
                                pane.scroll_idx -= 3;
                            }
                        },
                        Keycode::PageDown => {
                            if pane.scroll_idx > app.buffers[app.buffer_idx].contents.len() - 3 {
                                pane.scroll_idx = app.buffers[app.buffer_idx].contents.len();
                            } else {
                                pane.scroll_idx += 3;
                            }
                        },
                        Keycode::Return => {
                            pane.cursor_y += 1;
                            pane.cursor_x = 0;
                            pane.max_cursor_x = pane.cursor_x;
                            app.buffers[app.buffer_idx].contents.insert(pane.cursor_y, String::new());
                        }
                        Keycode::Backspace => {
                            if pane.cursor_x > 0 {
                                if pane.cursor_x < app.buffers[app.buffer_idx].contents[pane.cursor_y].len() {
                                    app.buffers[app.buffer_idx].contents[pane.cursor_y].remove(pane.cursor_x - 1);
                                }
                                pane.cursor_x -= 1;
                                pane.max_cursor_x = pane.cursor_x;
                                app.buffers[app.buffer_idx].is_dirty = true;
                            }
                        }
                        _ => {
                            if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                                match kc {
                                    Keycode::Q => {
                                        break 'mainloop;
                                    },
                                    Keycode::S => {
                                        println!("TODO: Save file");
                                    },
                                    Keycode::O => {
                                        println!("TODO: Open file");
                                    },
                                    _ => {},
                                }
                            }
                        }
                    }
                },
                _ => {}
            }
        }


        canvas.set_draw_color(Color::RGBA(200, 200, 200, 255));
        canvas.clear();

        for mut pane in &mut app.panes {

            // Smooth scrolling
            let target_scroll_offset = pane.scroll_idx as i32 * pane.line_height as i32;
            if pane.scroll_offset < target_scroll_offset {
                pane.scroll_offset += (f64::from(target_scroll_offset - pane.scroll_offset) / 3.0).ceil() as i32;
            } else if pane.scroll_offset > target_scroll_offset {
                pane.scroll_offset += (f64::from(target_scroll_offset - pane.scroll_offset) / 3.0).floor() as i32;
            }

            if let Some(b) = pane.buffer_id {
                let buffer = &app.buffers[b];
                canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
                // let p = &mut app.panes[app.pane_idx];
                let rect = rect!(pane.x, pane.y, pane.w, pane.h);
                canvas.fill_rect(rect).unwrap();

                let padding: i32 = 5;
                let bar_height: i32 = (app.line_height + padding * 2) as i32;
                // Draw the contents of the file and the cursor.
                for (i, entry) in buffer.contents.iter().enumerate() {

                    // Right-pad the string to allow the cursor to be rendered off the end of the line of
                    // text
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
                        bar_height + padding + i as i32 * pane.line_height - pane.scroll_offset,
                        &render_text);

                    // Draw the cursor if we're rendering the cursor line
                    if i == pane.cursor_y {
                        // If the cursor isn't at the beginning of the line, render the text before the
                        // cursor so we can measure its width.
                        let text_right = &render_text[..pane.cursor_x];
                        let (x, _) = pane.font.size_of(text_right).unwrap();
                        let rect = rect!(padding + x as i32, bar_height + padding + i as i32 * pane.line_height - pane.scroll_offset, 3, pane.line_height);
                        pane.fill_rect(&mut canvas, Color::RGBA(0, 0, 0, 255), rect);
                    }
                }

                // Draw bar
                let rect = rect!(0, 0, pane.w, bar_height);
                pane.fill_rect(&mut canvas, Color::RGBA(50, 50, 50, 255), rect);
                let dirty_text = if buffer.is_dirty { "*" } else { "" };
                // let mut dirty_text = "";
                // if buffer.is_dirty {
                //     dirty_text = "*";
                // }
                let bar_text = format!("{} {}", dirty_text, &buffer.name);
                pane.draw_text(&mut canvas, Color::RGBA(200, 200, 200, 255), padding, padding, &bar_text);
            }
        }

        canvas.present()
    }

    Ok(())
}
