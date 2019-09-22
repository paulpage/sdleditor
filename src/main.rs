extern crate sdl2;

// use std::path::PathBuf;
use std::fs;
use std::io::{BufReader, BufRead};

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::rect::Rect;
use sdl2::render::TextureQuery;
use sdl2::pixels::Color;

// static SCREEN_WIDTH : u32 = 800;
// static SCREEN_HEIGHT : u32 = 600;

// handle the annoying Rect i32
macro_rules! rect(
    ($x:expr, $y:expr, $w:expr, $h:expr) => (
        Rect::new($x as i32, $y as i32, $w as u32, $h as u32)
    )
);

// fn bounded_dec(target: usize) -> usize {
//     if target > 0 { target - 1 } else { target }
// }

// fn bounded_inc(target: usize, upper: usize) -> usize {
//     if target < upper { target + 1 } else { target }
// }

// fn read_dir<T: Into<PathBuf>>(path: T) -> Vec<String> {
//     fs::read_dir(&path.into())
//         .unwrap()
//         .map(|result| result.map(|entry| entry.path().display().to_string()).unwrap())
//         .map(|s| s.replacen("./", "", 1))
//         .collect()
// }

struct App {
    width: u32,
    height: u32,
    line_height: u32,
    buffer: Vec<String>,
    x: usize,
    y: usize,
    scroll_idx: usize,
    font_path: String,
    font_size: u16,

    // distance from top of document to top of viewport in pixels. Allows smooth scrolling.
    scroll_offset: i32,
}

// impl App {
//     fn set_dir<T: std::convert::AsRef<std::path::Path>>(&mut self, path: T) {
//         std::env::set_current_dir(path).unwrap();
//         self.contents = read_dir(".");
//         self.scroll_idx = 0;
//         self.selected_idx = 0;
//         self.search.clear();
//     }
// }

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
    let (width, height) = canvas.window().size();
    let mut app = App {
        width: width,
        height: height,
        line_height: 16, //TODO
        buffer: Vec::new(),
        x: 0,
        y: 0,
        scroll_idx: 0,
        font_path: "data/LiberationSans-Regular.ttf".to_string(),
        font_size: 16,
        scroll_offset: 0,
    };
    let font = ttf_context.load_font(app.font_path, app.font_size).unwrap();

    // app.buffer.push(String::new());
    let file = fs::File::open("src/main.rs").unwrap();
    let reader = BufReader::new(file);
    for line in reader.lines() {
        app.buffer.push(line.unwrap());
    }

    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::Quit {..} => break 'mainloop,
                Event::TextInput { text, .. } => {
                    app.buffer[app.y].insert_str(app.x, &text);
                    app.x += 1;
                },
                Event::MouseWheel { y, .. } => {
                    let candidate = app.scroll_idx as i32 - (y * 3);
                    if candidate < 0 {
                        app.scroll_idx = 0;
                    } else if candidate > app.buffer.len() as i32 {
                        app.scroll_idx = app.buffer.len();
                    } else {
                        app.scroll_idx = candidate as usize;
                    }
                }
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    match kc {
                        Keycode::Up => {
                            if app.y > 0 { app.y -= 1; }
                        },
                        Keycode::Down => {
                            app.y += 1;
                        },
                        Keycode::Left => {
                            if app.x > 0 { app.x -= 1; }
                        },
                        Keycode::Right => {
                            app.x += 1;
                        }
                        Keycode::PageUp => {
                            if app.scroll_idx < 3 {
                                app.scroll_idx = 0;
                                app.scroll_offset = 0;
                            } else {
                                app.scroll_idx -= 3;
                                // app.scroll_offset -= (3 * app.font_size as usize) as u32;
                            }
                        },
                        Keycode::PageDown => {
                            if app.scroll_idx > app.buffer.len() - 3 {
                                app.scroll_idx = app.buffer.len();
                                // app.scroll_offset = (app.buffer.len() * app.font_size as usize) as u32;
                            } else {
                                app.scroll_idx += 3;
                                // app.scroll_offset += (3 * app.font_size as u32) as u32;
                            }
                        },
                        Keycode::Backspace => {
                            if app.x > 0 {
                                if app.x < app.buffer[app.y].len() {
                                    app.buffer[app.y].remove(app.x - 1);
                                }
                                app.x -= 1;
                            }
                        }
                        // Keycode::Escape => {
                        //     app.search.clear();
                        // },
                        // Keycode::Backspace => {
                        //     app.set_dir("..");
                        // },
                        // Keycode::Return => {
                        //     let path =  &app.contents[app.selected_idx].clone();
                        //     if fs::metadata(path).unwrap().is_dir() {
                        //         app.set_dir(path);
                        //     } else {
                        //         println!("TODO: open file");
                        //     }
                        // },
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
                                    }
                                }
                            }
                            // let c = kc.name();
                            // if c.len() == 1 {
                            //     app.buffer[app.y].insert_str(app.x, &c);
                            //     app.x += 1;
                            //     // TODO: Handle shifted keys
                            //     // app.search.push_str(&c.to_lowercase());
                            //     // for (i, entry) in app.contents[app.selected_idx..].iter().enumerate() {
                            //     //     if entry.to_lowercase().starts_with(&app.search) {
                            //     //         println!("{} == {}", entry.to_lowercase(), &app.search);
                            //     //         app.selected_idx =  app.selected_idx + i;
                            //     //         break;
                            //     //     }
                            //     // }
                            // }
                        }
                    }
                },
                _ => {}
            }
        }

        // Smooth scrolling
        let target_scroll_offset = app.scroll_idx as i32 * app.font_size as i32;
        if app.scroll_offset < target_scroll_offset {
            app.scroll_offset += ((target_scroll_offset - app.scroll_offset) as f64 / 3.0).ceil() as i32;
        } else if app.scroll_offset > target_scroll_offset {
            app.scroll_offset += ((target_scroll_offset - app.scroll_offset) as f64 / 3.0).floor() as i32;
        }

        canvas.set_draw_color(Color::RGBA(200, 200, 200, 255));
        canvas.clear();

        // Draw the contents of the file and the cursor.
        let mut total_height: i32 = 0 - app.scroll_offset as i32;
        // for (i, entry) in app.buffer[app.scroll_idx..].iter().enumerate() {
        for (i, entry) in app.buffer.iter().enumerate() {

            // Right-pad the string to allow the cursor to be rendered off the end of the line of
            // text
            let mut render_text = entry.clone();
            if render_text.len() < app.x {
                for _ in render_text.len()..app.x {
                    render_text.push(' ');
                }
            }
            // Avoid trying to render an empty string, which is an error in SDL
            if render_text.len() == 0 {
                render_text.push(' ');
            }

            let padding = 5;
            
            // Render the full line of text
            let surface = font.render(&render_text).blended(Color::RGBA(40, 0, 0, 255)).unwrap();
            let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
            let TextureQuery { width, height, .. } = texture.query();
            let target = rect!(0 + padding, total_height + padding as i32, width, height);
            canvas.copy(&texture, None, Some(target))?;

            // Draw the cursor if we're rendering the cursor line
            // if app.scroll_idx + i == app.y {
            if i == app.y {
                // If the cursor isn't at the beginning of the line, render the text before the
                // cursor so we can measure its width.
                let text_right = &render_text[..app.x];
                let mut cursor_x = 0;
                if text_right.len() > 0 {
                    let surface = font.render(&text_right).blended(Color::RGBA(40, 0, 0, 255)).unwrap();
                    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();
                    let info = texture.query();
                    cursor_x = info.width;
                }
                let cursor = rect!(cursor_x + padding, total_height + padding as i32, 3, height);
                canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
                canvas.fill_rect(cursor)?;
            }

            total_height += height as i32;
        }

        canvas.present()
    }

    Ok(())
}
