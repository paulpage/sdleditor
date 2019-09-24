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
    cursor_y: usize,
    font: Font<'a, 'a>,
    line_height: i32,
    buffer_id: Option<usize>,
}

impl<'a> Pane<'a> {

    fn fill_rect(&mut self, canvas: &mut WindowCanvas, color: Color, rect: Rect) {
        canvas.set_draw_color(color);
        let x = self.x + max(rect.x, 0);
        let y = self.y + max(rect.y, 0);
        let w = min(self.w as i32 - rect.x, rect.w);
        let h = min(self.h as i32 - rect.y, rect.h);
        canvas.fill_rect(rect!(x, y, w, h));
    }

    // Draw the given text on the given canvas with the given color.
    // Upper left corner of the text will be the x, y of the rect,
    // and text outside the width and height of the rect will be cut off.
    fn draw_text(&mut self, canvas: &mut WindowCanvas, color: Color, x: i32, y: i32, text: &str) {
        if y > 0 && x > 0 {
            let surface = self.font.render(text).blended(color).unwrap();
            let texture_creator = canvas.texture_creator();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            let TextureQuery { width: w, height: h, .. } = texture.query();
            let w = min(self.w as i32 - x, w as i32);
            let h = min(self.h as i32 - y, h as i32);
            let source = rect!(0, 0, w, h);
            let target = rect!(self.x + x, self.y + y, w, h);
            canvas.copy(&texture, Some(source), Some(target));
        }
    }
}

struct App<'a> {
    line_height: i32,
    buffers: Vec<Buffer>,
    panes: Vec<Pane<'a>>,
    scroll_idx: usize,
    font_path: String,
    font_size: u16,

    // distance from top of document to top of viewport in pixels. Allows smooth scrolling.
    scroll_offset: i32,

    buffer_idx: usize,
    pane_idx: usize,
}

impl<'a> App<'a> {
    fn active_buffer(&mut self) -> &mut Buffer {
        &mut self.buffers[self.buffer_idx]
    }
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
    let texture_creator = canvas.texture_creator();

    // Initialize app
    let mut app = App {
        line_height: 16,
        buffers: Vec::new(),
        panes: Vec::new(),
        buffer_idx: 0,
        pane_idx: 0,
        scroll_idx: 0,
        font_path: "data/LiberationSans-Regular.ttf".to_string(),
        font_size: 16,
        scroll_offset: 0,
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
        cursor_y: 0,
        font: font,
        line_height: 0,
        buffer_id: Some(0),
    });
    app.panes[app.pane_idx].line_height = app.panes[app.pane_idx].font.height();

    app.line_height = app.panes[app.pane_idx].font.height();

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::Quit {..} => break 'mainloop,
                Event::TextInput { text, .. } => {
                    app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].insert_str(app.panes[app.pane_idx].cursor_x, &text);
                    app.panes[app.pane_idx].cursor_x += 1;
                    app.buffers[app.buffer_idx].is_dirty = true;
                },
                Event::MouseButtonDown { x, y, .. } => {
                    let current_line = &app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y];
                    // Measure the length of each substring of the line until we get one that's
                    // bigger than the x position of the mouse
                    // let mut screen_x = 0;
                    // let mut char_x = 0;
                    // while char_x < x && (screen_x as usize) < app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].len() {
                    //     let (cx, _) = font.size_of(&app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y][..screen_x]).unwrap();
                    //     char_x = cx as i32;
                    //     screen_x += 1;
                    // }
                    // TODO fix the sizing situation (padding, bars, etc.)
                    // let screen_y = ((y as f64 - app.line_height as f64 - 15 as f64) / app.line_height as f64).floor() as usize;
                    // app.panes[app.pane_idx].cursor_x = screen_x;
                    // app.panes[app.pane_idx].cursor_y = app.scroll_idx + screen_y;
                },
                Event::MouseWheel { y, .. } => {
                    let candidate = app.scroll_idx as i32 - (y * 3);
                    if candidate < 0 {
                        app.scroll_idx = 0;
                    } else if candidate > app.buffers[app.buffer_idx].contents.len() as i32 {
                        app.scroll_idx = app.buffers[app.buffer_idx].contents.len();
                    } else {
                        app.scroll_idx = candidate as usize;
                    }
                },
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    match kc {
                        Keycode::Up => {
                            if app.panes[app.pane_idx].cursor_y > 0 { app.panes[app.pane_idx].cursor_y -= 1; }
                        },
                        Keycode::Down => {
                            app.panes[app.pane_idx].cursor_y += 1;
                        },
                        Keycode::Left => {
                            if app.panes[app.pane_idx].cursor_x > 0 { app.panes[app.pane_idx].cursor_x -= 1; }
                        },
                        Keycode::Right => {
                            app.panes[app.pane_idx].cursor_x += 1;
                        }
                        Keycode::PageUp => {
                            if app.scroll_idx < 3 {
                                app.scroll_idx = 0;
                                app.scroll_offset = 0;
                            } else {
                                app.scroll_idx -= 3;
                            }
                        },
                        Keycode::PageDown => {
                            if app.scroll_idx > app.buffers[app.buffer_idx].contents.len() - 3 {
                                app.scroll_idx = app.buffers[app.buffer_idx].contents.len();
                            } else {
                                app.scroll_idx += 3;
                            }
                        },
                        Keycode::Return => {
                            app.panes[app.pane_idx].cursor_y += 1;
                            app.panes[app.pane_idx].cursor_x = 0;
                            app.buffers[app.buffer_idx].contents.insert(app.panes[app.pane_idx].cursor_y, String::new());
                        }
                        Keycode::Backspace => {
                            if app.panes[app.pane_idx].cursor_x > 0 {
                                if app.panes[app.pane_idx].cursor_x < app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].len() {
                                    app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].remove(app.panes[app.pane_idx].cursor_x - 1);
                                }
                                app.panes[app.pane_idx].cursor_x -= 1;
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

        // Smooth scrolling
        let target_scroll_offset = app.scroll_idx as i32 * app.line_height as i32;
        if app.scroll_offset < target_scroll_offset {
            app.scroll_offset += ((target_scroll_offset - app.scroll_offset) as f64 / 3.0).ceil() as i32;
        } else if app.scroll_offset > target_scroll_offset {
            app.scroll_offset += ((target_scroll_offset - app.scroll_offset) as f64 / 3.0).floor() as i32;
        }

        canvas.set_draw_color(Color::RGBA(200, 200, 200, 255));
        canvas.clear();

        for mut pane in &mut app.panes {
            if let Some(b) = pane.buffer_id {
                let buffer = &app.buffers[b];
                canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
                // let p = &mut app.panes[app.pane_idx];
                let rect = rect!(pane.x, pane.y, pane.w, pane.h);
                canvas.fill_rect(rect);

                let padding: i32 = 5;
                let bar_height: i32 = (app.line_height + padding * 2) as i32;
                let base_x: i32 = pane.x;
                let base_y: i32 = pane.y + bar_height;
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
                    // Avoid trying to render an empty string, which is an error in SDL
                    if render_text.len() == 0 {
                        render_text.push(' ');
                    }

                    // Render the full line of text
                    let height = pane.line_height;
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding,
                        bar_height + padding + i as i32 * pane.line_height - app.scroll_offset,
                        &render_text);

                    // Draw the cursor if we're rendering the cursor line
                    if i == pane.cursor_y {
                        // If the cursor isn't at the beginning of the line, render the text before the
                        // cursor so we can measure its width.
                        let text_right = &render_text[..pane.cursor_x];
                        let mut cursor_x = 0;
                        let (cursor_x, _) = pane.font.size_of(text_right).unwrap();
                        let rect = rect!(padding + cursor_x as i32, bar_height + padding + i as i32 * pane.line_height - app.scroll_offset, 3, pane.line_height);
                        pane.fill_rect(&mut canvas, Color::RGBA(0, 0, 0, 255), rect);
                    }
                }

                // Draw bar
                let rect = rect!(0, 0, pane.w, bar_height);
                pane.fill_rect(&mut canvas, Color::RGBA(50, 50, 50, 255), rect);
                let mut dirty_text = "";
                if buffer.is_dirty {
                    dirty_text = "*";
                }
                let bar_text = format!("{} {}", dirty_text, &buffer.name);
                pane.draw_text(&mut canvas, Color::RGBA(200, 200, 200, 255), padding, padding, &bar_text);
            }
        }

        canvas.present()
    }

    Ok(())
}
