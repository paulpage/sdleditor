extern crate sdl2;

// use std::path::PathBuf;
use std::fs;
use std::io::{BufReader, BufRead};
use std::env;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::pixels::Color;

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

struct Pane {
    x: i32 ,
    y: i32,
    w: u32,
    h: u32,
    cursor_x: usize,
    cursor_y: usize,
}

// impl Buffer {
// }

struct App {
    line_height: i32,
    buffers: Vec<Buffer>,
    panes: Vec<Pane>,
    scroll_idx: usize,
    font_path: String,
    font_size: u16,

    // distance from top of document to top of viewport in pixels. Allows smooth scrolling.
    scroll_offset: i32,

    buffer_idx: usize,
    pane_idx: usize,
}

impl App {
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
        }
    }
    let (width, height) = canvas.window().size();
    app.panes.push(Pane {
        x: 50,
        y: 50,
        w: width - 100,
        h: height - 100,
        cursor_x: 0,
        cursor_y: 0,
    });

    let font = ttf_context.load_font(&app.font_path, app.font_size).unwrap();
    app.line_height = font.height();

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
                    let mut screen_x = 0;
                    let mut char_x = 0;
                    while char_x < x && (screen_x as usize) < app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y].len() {
                        let (cx, _) = font.size_of(&app.buffers[app.buffer_idx].contents[app.panes[app.pane_idx].cursor_y][..screen_x]).unwrap();
                        char_x = cx as i32;
                        screen_x += 1;
                    }
                    // TODO fix the sizing situation (padding, bars, etc.)
                    let screen_y = ((y as f64 - app.line_height as f64 - 15 as f64) / app.line_height as f64).floor() as usize;
                    app.panes[app.pane_idx].cursor_x = screen_x;
                    app.panes[app.pane_idx].cursor_y = app.scroll_idx + screen_y;
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

        canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
        let p = &app.panes[app.pane_idx];
        let rect = rect!(p.x, p.y, p.w, p.h);
        canvas.fill_rect(rect);

        let padding: i32 = 5;
        let bar_height: i32 = (app.line_height + padding * 2) as i32;
        let base_x: i32 = app.panes[app.pane_idx].x;
        let base_y: i32 = app.panes[app.pane_idx].y + bar_height;
        // Draw the contents of the file and the cursor.
        for (i, entry) in app.buffers[app.buffer_idx].contents.iter().enumerate() {

            // Right-pad the string to allow the cursor to be rendered off the end of the line of
            // text
            let mut render_text = entry.clone();
            if render_text.len() < app.panes[app.pane_idx].cursor_x {
                for _ in render_text.len()..app.panes[app.pane_idx].cursor_x {
                    render_text.push(' ');
                }
            }
            // Avoid trying to render an empty string, which is an error in SDL
            if render_text.len() == 0 {
                render_text.push(' ');
            }

            // Render the full line of text
            let surface = font.render(&render_text).blended(Color::RGBA(40, 0, 0, 255)).unwrap();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            let TextureQuery { width, height, .. } = texture.query();
            let target = rect!(base_x + padding, base_y + padding + i as i32 * app.line_height - app.scroll_offset, width, height);
            canvas.copy(&texture, None, Some(target))?;

            // Draw the cursor if we're rendering the cursor line
            if i == app.panes[app.pane_idx].cursor_y {
                // If the cursor isn't at the beginning of the line, render the text before the
                // cursor so we can measure its width.
                let text_right = &render_text[..app.panes[app.pane_idx].cursor_x];
                let mut cursor_x = 0;
                let (cursor_x, _) = font.size_of(text_right).unwrap();
                let cursor = rect!(
                    base_x + padding + cursor_x as i32,
                    base_y + padding * app.line_height - app.scroll_offset,
                    3,
                    height);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                canvas.fill_rect(cursor)?;
            }
        }

        // Draw bar
        let rect = rect!(base_x, base_y - bar_height, app.panes[app.pane_idx].w, bar_height);
        canvas.set_draw_color(Color::RGBA(50, 50, 50, 255));
        canvas.fill_rect(rect)?;
        let mut dirty_text = "";
        if app.buffers[app.buffer_idx].is_dirty {
            dirty_text = "*";
        }
        let bar_text = format!("{} {}", dirty_text, &app.buffers[app.buffer_idx].name);
        let surface = font.render(&bar_text).blended(Color::RGBA(200, 200, 200, 255)).unwrap();
        let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
        let TextureQuery { width: w, height: h, .. } = texture.query();
        let target = rect!(base_x + padding, base_y - bar_height + padding, w, h);
        canvas.copy(&texture, None, Some(target))?;

        canvas.present()
    }

    Ok(())
}
