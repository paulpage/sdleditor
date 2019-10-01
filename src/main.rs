extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

mod pane;
use pane::{Pane, PaneType};

mod buffer;
use buffer::{Buffer};

fn handle_buffer_event(pane: &mut Pane, mut buffer: &mut Buffer, event: Event) {
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
            let mut y_idx = max(((f64::from(y)
                              - f64::from(pane.y)
                              - f64::from(padding)
                              - f64::from(bar_height))
                             / f64::from(pane.line_height))
                .floor() as i32, 0) as usize
                + pane.scroll_idx;
            y_idx = min(y_idx, buffer.contents.len() - 1);
            let max_x_idx = buffer.contents[y_idx].len();

            pane.cursor_y = y_idx;

            let mut length = pane.x + padding;
            let mut x_idx = 0;
            let mut last_length = length;
            while length < x && (x_idx as usize) < max_x_idx {
                last_length = length;
                let (char_x, _) =  pane.font
                    .size_of(&buffer.contents[pane.cursor_y].chars().nth(x_idx).unwrap().to_string())
                    .unwrap();
                length += char_x as i32;
                x_idx += 1;
            }
            if (last_length as i32 - x as i32).abs() > (length as i32 - x as i32).abs() {
                x_idx += 1;
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
                Keycode::Up => pane.cursor_up(1, &buffer),
                Keycode::Down => pane.cursor_down(1, &buffer),
                Keycode::Left => pane.cursor_left(1, &buffer),
                Keycode::Right => pane.cursor_right(1, &buffer),
                Keycode::PageUp => pane.scroll_up(3),
                Keycode::PageDown => pane.scroll_down(3, &buffer),
                Keycode::Return => pane.break_line(&mut buffer),
                Keycode::Backspace => pane.remove_char(&mut buffer),
                _ => {}
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
        2 => buffers.push(Buffer::from_path(args[1].to_string())),
        _ => buffers.push(Buffer::new()),
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
                                break 'mainloop;
                            }
                            Keycode::S => {
                                if let Some(b) = panes[pane_idx].buffer_id {
                                    buffers[b].save();
                                }
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
                let buffer: &mut Buffer = &mut buffers[b];
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

                let padding: i32 = 5;
                let bar_height: i32 = line_height as i32 + padding * 2;
                // Draw the contents of the file and the cursor.

                for (i, entry) in buffer.contents[first_line..last_line].iter().enumerate() {

                    let midpoint = min(pane.cursor_x, entry.len());

                    let midpoint_width = pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding,
                        bar_height + padding + (i as i32 + first_line as i32) * line_height as i32 - scroll_offset,
                        &entry[0..midpoint]);
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(40, 0, 0, 255),
                        padding + midpoint_width,
                        bar_height + padding + (i + first_line) as i32 * line_height as i32 - scroll_offset,
                        &entry[midpoint..entry.len()]);

                    // Draw the cursor if we're rendering the cursor line
                    if first_line + i == pane.cursor_y {
                        let rect = Rect::new(
                            padding + midpoint_width as i32,
                            bar_height + padding + (i + first_line) as i32 * line_height as i32 - scroll_offset as i32,
                            2,
                            line_height as u32
                        );
                        pane.fill_rect(&mut canvas, Color::RGBA(0, 0, 0, 255), rect);
                    }

                    // Draw bar
                    let rect = Rect::new(0, 0, pane.w, bar_height as u32);
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
        }

        sleep(Duration::from_millis(5));
        canvas.present(); 
    }
}
