extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

mod pane;
use pane::{Pane, PaneType};

struct Buffer {
    name: String,
    contents: Vec<String>,
    is_dirty: bool,
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

fn handle_buffer_event(pane: &mut Pane, buffer: &mut Buffer, event: Event) {
    match event {
        Event::TextInput { text, .. } => {
            buffer.contents[pane.cursor_y].insert_str(pane.cursor_x, &text);
            pane.cursor_x += text.len();
            pane.max_cursor_x = pane.cursor_x;
            buffer.is_dirty = true;
        }
        Event::MouseButtonDown { x, y, .. } => {
            let bar_height: u32 = (pane.line_height + 5 * 2) as u32;
            let padding = 5;
            let mut y_idx = ((f64::from(y)
                              - f64::from(pane.y)
                              - f64::from(padding)
                              - f64::from(bar_height))
                             / f64::from(pane.line_height))
                .floor() as usize
                + pane.scroll_idx;
            y_idx = min(y_idx, buffer.contents.len() - 1);
            let max_x_idx = buffer.contents[y_idx].len();

            pane.cursor_y = y_idx;
            // Measure the length of each substring of the line until we get one that's
            // bigger than the x position of the mouse
            let mut x_idx = 0;
            let mut char_x = pane.x + padding;
            let mut last_char_x = char_x;
            while char_x < x && (x_idx as usize) < max_x_idx + 1 {
                let (cx, _) = pane.font
                    .size_of(&buffer.contents[pane.cursor_y][..x_idx])
                    .unwrap();
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
        }
        Event::MouseWheel { y, .. } => {
            let candidate = pane.scroll_idx as i32 - (y * 3);
            if candidate < 0 {
                pane.scroll_idx = 0;
            } else if candidate > buffer.contents.len() as i32 {
                pane.scroll_idx = buffer.contents.len();
            } else {
                pane.scroll_idx = candidate as usize;
            }
        }
        Event::KeyDown {
            keycode: Some(kc),
            keymod,
            ..
        } => {
            match kc {
                Keycode::Up => {
                    if pane.cursor_y > 0 {
                        pane.cursor_y -= 1;
                        pane.cursor_x = max(
                            min(pane.cursor_x, buffer.contents[pane.cursor_y].len()),
                            min(pane.max_cursor_x, buffer.contents[pane.cursor_y].len()),
                            );
                    }
                }
                Keycode::Down => {
                    if pane.cursor_y < buffer.contents.len() {
                        pane.cursor_y += 1;
                        pane.cursor_x = max(
                            min(pane.cursor_x, buffer.contents[pane.cursor_y].len()),
                            min(pane.max_cursor_x, buffer.contents[pane.cursor_y].len()),
                            );
                    }
                }
                Keycode::Left => {
                    if pane.cursor_x > 0 {
                        pane.cursor_x -= 1;
                        pane.max_cursor_x = pane.cursor_x;
                    }
                }
                Keycode::Right => {
                    if pane.cursor_x < buffer.contents[pane.cursor_y].len() {
                        pane.cursor_x += 1;
                        pane.max_cursor_x = pane.cursor_x;
                    }
                }
                Keycode::PageUp => {
                    if pane.scroll_idx < 3 {
                        pane.scroll_idx = 0;
                        pane.scroll_offset = 0;
                    } else {
                        pane.scroll_idx -= 3;
                    }
                }
                Keycode::PageDown => {
                    if pane.scroll_idx > buffer.contents.len() - 3 {
                        pane.scroll_idx = buffer.contents.len();
                    } else {
                        pane.scroll_idx += 3;
                    }
                }
                Keycode::Return => {
                    pane.cursor_y += 1;
                    pane.cursor_x = 0;
                    pane.max_cursor_x = pane.cursor_x;
                    buffer.contents.insert(pane.cursor_y, String::new());
                }
                Keycode::Backspace => {
                    if pane.cursor_x > 0 {
                        if pane.cursor_x <= buffer.contents[pane.cursor_y].len() {
                            buffer.contents[pane.cursor_y].remove(pane.cursor_x - 1);
                        }
                        pane.cursor_x -= 1;
                        pane.max_cursor_x = pane.cursor_x;
                        buffer.is_dirty = true;
                    }
                }
                _ => {
                }
            }
        }
        _ => {}
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
    panes.push(Pane::new(
            100,
            100,
            400,
            400,
            ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap(),
            PaneType::Buffer,
            Some(0)));
    // panes[pane_idx].line_height = panes[pane_idx].font.height();
    // panes[pane_idx].line_height = font_manager.font.height();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown {
                    keycode: Some(kc),
                    keymod,
                    ..
                } => {
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
                                panes.push(Pane::new(
                                        120,
                                        120,
                                        400,
                                        400,
                                        ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap(),
                                        PaneType::FileManager,
                                        None));
                            }
                            _ => {
                                if let Some(b) = panes[pane_idx].buffer_id {
                                    handle_buffer_event(&mut panes[pane_idx], &mut buffers[b], event);
                                }
                            }
                        }
                    } else {
                        if let Some(b) = panes[pane_idx].buffer_id {
                            handle_buffer_event(&mut panes[pane_idx], &mut buffers[b], event);
                        }
                    }

		}
                _ => {
                    if let Some(b) = panes[pane_idx].buffer_id {
                        handle_buffer_event(&mut panes[pane_idx], &mut buffers[b], event);
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
                let buffer: &Buffer = &buffers[b];
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
                    let line_width = pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding as i32,
                        bar_height as i32 + padding as i32 + (i as i32 + first_line as i32) * line_height as i32
                        - scroll_offset,
                        &entry,
                    );

                    // Draw the cursor if we're rendering the cursor line
                    if i == pane.cursor_y {
                        let rect = Rect::new(
                            padding as i32 + line_width as i32,
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
