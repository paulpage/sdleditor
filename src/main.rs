extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

mod pane;
use pane::{Pane, PaneType};

mod buffer;
use buffer::{Buffer};

fn handle_buffer_event(pane: &mut Pane, mut buffer: &mut Buffer, event: Event) {
    match event {
        Event::Window { win_event, .. } => {
            match win_event {
                WindowEvent::Resized(w, h) => {
                    pane.w = max(0, w - 40) as u32;
                    pane.h = max(0, h - 40) as u32;
                }
                _ => {}
            }
        }
        Event::TextInput { text, .. } => {
            buffer.contents[pane.cursor_y].insert_str(pane.cursor_x, &text);
            pane.cursor_x += text.len();
            pane.max_cursor_x = pane.cursor_x;
            buffer.is_dirty = true;
            pane.set_selection(false);
        }
        Event::MouseButtonDown { x, y, .. } => {
            let (x_idx, y_idx) = pane.get_position_from_screen(x, y, buffer);
            pane.cursor_x = x_idx;
            pane.cursor_y = y_idx;
            pane.set_selection(false);
        }
        Event::MouseMotion { mousestate, x, y, .. } => {
            if mousestate.is_mouse_button_pressed(MouseButton::Left) {
            let (x_idx, y_idx) = pane.get_position_from_screen(x, y, buffer);
            pane.cursor_x = x_idx;
            pane.cursor_y = y_idx;
            pane.set_selection(true);
            }
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
            if keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::LSHIFTMOD) {
                match kc {
                    Keycode::Up => pane.cursor_up(1, &buffer, true),
                    Keycode::Down => pane.cursor_down(1, &buffer, true),
                    Keycode::Left => pane.cursor_left(1, &buffer, true),
                    Keycode::Right => pane.cursor_right(1, &buffer, true),
                    _ => {}
                }
            } else {
                match kc {
                    Keycode::Up => pane.cursor_up(1, &buffer, false),
                    Keycode::Down => pane.cursor_down(1, &buffer, false),
                    Keycode::Left => pane.cursor_left(1, &buffer, false),
                    Keycode::Right => pane.cursor_right(1, &buffer, false),
                    Keycode::PageUp => pane.scroll_up(3),
                    Keycode::PageDown => pane.scroll_down(3, &buffer),
                    Keycode::Return => pane.break_line(&mut buffer),
                    Keycode::Backspace => pane.remove_selection(&mut buffer),
                    _ => {}
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
        .resizable()
        .maximized()
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

    let (width, height) = canvas.window().size();
    panes.push(Pane::new(
            20,
            20,
            max(0, width as i32 - 40) as u32,
            max(0, height as i32 - 40) as u32,
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
                        if keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::LSHIFTMOD) {
                            match kc {
                                Keycode::Backslash => {
                                    if let Some(b) = panes[pane_idx].buffer_id {
                                        buffers[b].print();
                                        break 'mainloop;
                                    }
                                }
                                Keycode::Z => {
                                    if let Some(b) = panes[pane_idx].buffer_id {
                                        buffers[b].redo();
                                    }
                                }
                                _ => {}
                            }
                        } else {
                            match kc {
                                Keycode::Q => break 'mainloop,
                                Keycode::Z => {
                                    if let Some(b) = panes[pane_idx].buffer_id {
                                        buffers[b].undo();
                                    }
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

        let (width, height) = canvas.window().size();
        canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
        canvas.clear();

        canvas.set_draw_color(Color::RGBA(20, 20, 20, 255));

        for mut pane in &mut panes {
            pane.draw(&mut canvas);
            let line_height = pane.line_height as i32;
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
                canvas.set_draw_color(Color::RGBA(40, 40, 40, 255));
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

                for (i, entry) in buffer.contents[first_line..last_line].iter().enumerate()
                    .map(|(i, entry)| (i + first_line, entry)) {

                    let midpoint = min(pane.cursor_x, entry.len());
                    let line_y = bar_height + padding + i as i32 * line_height as i32 - scroll_offset;

                    // Draw the selection
                    let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = pane.get_selection();
                    if i >= sel_start_y && i <= sel_end_y {
                        let mut x1: u32 = 0;
                        let mut x2: u32 = pane.text_length(&buffer.contents[i]);
                        if buffer.contents[i].len() > 0 {
                            if i == sel_start_y {
                                x1 = pane.text_length(&buffer.contents[i][..sel_start_x]);
                            }
                            if i == sel_end_y {
                                x2 = pane.text_length(&buffer.contents[i][..sel_end_x]);
                            }
                        }
                        let color = Color::RGBA(146, 131, 116, 0);
                        let rect = Rect::new(
                            padding as i32 + x1 as i32,
                            line_y,
                            (x2 - x1) as u32,
                            line_height as u32);
                        pane.fill_rect(&mut canvas, color, rect);
                    }

                    // Draw the text
                    let midpoint_width = pane.draw_text(
                        &mut canvas,
                        Color::RGBA(251, 241, 199, 255),
                        padding,
                        line_y,
                        &entry[0..midpoint]);
                    let text_length = midpoint_width + pane.draw_text(
                        &mut canvas,
                        Color::RGBA(251, 241, 199, 255),
                        padding + midpoint_width,
                        line_y,
                        &entry[midpoint..entry.len()]);

                    // Draw the cursor
                    if i == pane.cursor_y {
                        let rect = Rect::new(
                            padding + midpoint_width as i32,
                            line_y,
                            2,
                            line_height as u32
                        );
                        pane.fill_rect(&mut canvas, Color::RGBA(235, 219, 178, 255), rect);
                    }

                    // Draw the bar
                    let rect = Rect::new(0, 0, pane.w, bar_height as u32);
                    pane.fill_rect(&mut canvas, Color::RGBA(80, 73, 69, 255), rect);
                    let dirty_text = if buffer.is_dirty { "*" } else { "" };
                    let bar_text = format!("{} {}", dirty_text, &buffer.name);
                    pane.draw_text(
                        &mut canvas,
                        Color::RGBA(251, 241, 199, 255),
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
