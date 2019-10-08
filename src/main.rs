extern crate clipboard;
extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::Mod;
use sdl2::mouse::MouseButton;
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::WindowCanvas;

extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

mod pane;
use pane::{Pane, PaneType};

mod buffer;
use buffer::Buffer;

mod file_manager;
use file_manager::FileManager;

fn draw(
    panes: &mut Vec<Pane>,
    buffers: &mut Vec<Buffer>,
    pane_idx: usize,
    mut canvas: &mut WindowCanvas,
) {
    canvas.set_draw_color(Color::RGBA(0, 0, 0, 255));
    canvas.clear();

    let padding: i32 = 5;
    for (j, pane) in &mut panes.iter_mut().enumerate() {
        pane.draw(&mut canvas, padding, j == pane_idx);
        let bar_height: i32 = pane.line_height as i32 + padding * 2;

        let buffer = &buffers[pane.buffer_id];

        // We only want to render the lines that are actually on the screen.
        let (first_line, last_line) = pane.get_lines_on_screen(&buffer);

        // Draw the contents of the file and the cursor.
        for (i, entry) in buffer.contents[first_line..last_line]
            .iter()
            .enumerate()
            .map(|(i, entry)| (i + first_line, entry))
        {
            let uentry =
                UnicodeSegmentation::graphemes(entry.as_str(), true).collect::<Vec<&str>>();
            let midpoint = min(pane.cursor_x, uentry.len());
            let line_y =
                bar_height + padding * 2 + i as i32 * pane.line_height as i32 - pane.scroll_offset;

            // Draw the selection
            let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = pane.get_selection();
            if i >= sel_start_y && i <= sel_end_y {
                let mut x1: u32 = 0;
                let mut x2: u32 = pane.text_length(&buffer.contents[i]);
                if !buffer.contents[i].is_empty() {
                    if i == sel_start_y {
                        x1 = pane.text_length(
                            &buffer.line_graphemes(i)[..sel_start_x].concat().to_string(),
                        );
                    }
                    if i == sel_end_y {
                        x2 = pane.text_length(
                            &buffer.line_graphemes(i)[..sel_end_x].concat().to_string(),
                        );
                    }
                }
                let color = Color::RGBA(146, 131, 116, 0);
                if x2 > x1 {
                    let rect = Rect::new(
                        padding * 2 + x1 as i32,
                        line_y,
                        (x2 - x1) as u32,
                        pane.line_height as u32,
                    );
                    pane.fill_rect(&mut canvas, color, rect);
                }
            }

            // Draw the text
            let midpoint_width = pane.draw_text(
                &mut canvas,
                Color::RGBA(251, 241, 199, 255),
                padding * 2,
                line_y,
                &uentry[0..midpoint].concat(),
            );
            pane.draw_text(
                &mut canvas,
                Color::RGBA(251, 241, 199, 255),
                padding * 2 + midpoint_width,
                line_y,
                &uentry[midpoint..].concat(),
            );

            // Draw the cursor
            if j == pane_idx && i == pane.cursor_y {
                let rect = Rect::new(
                    padding * 2 + midpoint_width as i32,
                    line_y,
                    2,
                    pane.line_height as u32,
                );
                pane.fill_rect(&mut canvas, Color::RGBA(235, 219, 178, 255), rect);
            }
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

fn handle_local_keystroke(pane: &mut Pane, buffer: &mut Buffer, kstr: &str) -> bool {
    match kstr {
        "Up" => pane.cursor_up(1, buffer, false),
        "Down" => pane.cursor_down(1, buffer, false),
        "Left" => pane.cursor_left(buffer, false),
        "Right" => pane.cursor_right(buffer, false),
        "PageUp" => pane.scroll_up(3),
        "PageDown" => pane.scroll_down(3, buffer),
        "Return" => pane.break_line(buffer),
        "Backspace" => pane.remove_selection(buffer),
        "S-Up" => pane.cursor_up(1, buffer, true),
        "S-Down" => pane.cursor_down(1, buffer, true),
        "S-Left" => pane.cursor_left(buffer, true),
        "S-Right" => pane.cursor_right(buffer, true),
        "C-A" => pane.select_all(buffer),
        "C-C" => pane.clipboard_copy(buffer),
        "C-S" => buffer.save(),
        "C-V" => pane.clipboard_paste(buffer),
        "C-X" => pane.clipboard_cut(buffer),
        "C-Z" => buffer.undo(),
        "C-S-Z" => buffer.redo(),
        "C-S-\\" => {
            buffer.print();
            return true;
        }
        _ => {}
    }
    false
}

fn insert_text(pane: &mut Pane, buffer: &mut Buffer, text: String) {
    let (x1, y1, x2, y2) = pane.get_selection();
    buffer.delete_text(x1, y1, x2, y2);
    buffer.insert_text(pane.cursor_x, pane.cursor_y, text.clone());
    pane.cursor_x += text.len();
    pane.set_selection(false);
}

fn set_selection_from_screen(
    pane: &mut Pane,
    buffer: &mut Buffer,
    x: i32,
    y: i32,
    extend_selection: bool,
) {
    let (x_idx, y_idx) = pane.get_position_from_screen(x, y, buffer);
    pane.cursor_x = x_idx;
    pane.cursor_y = y_idx;
    pane.set_selection(extend_selection);
}

fn scroll(pane: &mut Pane, buffer: &Buffer, y: i32) {
    let candidate = pane.scroll_idx as i32 - (y * 3);
    if candidate < 0 {
        pane.scroll_idx = 0;
    } else if candidate > buffer.len() as i32 {
        pane.scroll_idx = buffer.len();
    } else {
        pane.scroll_idx = candidate as usize;
    }
}

fn next<T>(list: &[T], idx: usize) -> usize {
    if idx < list.len() - 1 {
        idx + 1
    } else {
        0
    }
}

fn prev<T>(list: &[T], idx: usize) -> usize {
    if idx > 0 {
        idx - 1
    } else {
        list.len() - 1
    }
}

fn arrange(canvas: &WindowCanvas, panes: &mut Vec<Pane>) {
    let (w, h) = canvas.window().size();

    // let mut x = 0;
    // let mut y = 0;
    // for mut pane in &mut panes.iter_mut() {
    //     pane.x = x;
    //     pane.y = y;
    //     pane.w = 400;
    //     pane.h = 400;
    //     x += 20;
    //     y += 20;
    // }

    let padding = 5;
    let pane_width = (f64::from(w) / panes.len() as f64).floor() as u32;
    let pane_height = h;
    let mut x = 0;
    let y = 0;
    for mut pane in &mut panes.iter_mut() {
        pane.x = x + padding;
        pane.y = y + padding;
        pane.w = max(0, pane_width as i32 - (padding * 2) as i32) as u32;
        pane.h = max(0, pane_height as i32 - (padding * 2) as i32) as u32;
        x += pane_width as i32;
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
        .map_err(|e| e.to_string())
        .unwrap();
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();

    let mut buffers: Vec<Buffer> = Vec::new();
    let mut panes: Vec<Pane> = Vec::new();
    let mut pane_idx = 0;

    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        for arg in &args[1..] {
            buffers.push(Buffer::from_path(arg.to_string()));
        }
    } else {
        buffers.push(Buffer::new());
    }

    panes.push(Pane::new(
        ttf_context
            .load_font("data/LiberationSans-Regular.ttf", 16)
            .unwrap(),
        PaneType::Buffer,
        0,
    ));
    arrange(&canvas, &mut panes);

    let mut ctrl_pressed = false;
    let mut alt_pressed = false;
    let mut fm = FileManager::new();
    // Don't redraw unless we have to
    let mut is_dirty;

    'mainloop: loop {
        // let t = Instant::now();
        is_dirty = false;
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            is_dirty = true;
            if let Event::KeyDown {
                keycode: Some(kc),
                keymod,
                ..
            } = event
            {
                let mut key_string = String::new();
                if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                    key_string.push_str("C-");
                    ctrl_pressed = true;
                }
                if keymod.contains(Mod::RALTMOD) || keymod.contains(Mod::LALTMOD) {
                    key_string.push_str("A-");
                    alt_pressed = true;
                }
                if keymod.contains(Mod::RSHIFTMOD) || keymod.contains(Mod::LSHIFTMOD) {
                    key_string.push_str("S-");
                }
                key_string.push_str(&kc.name());

                let kstr: &str = &key_string.clone();
                match kstr {
                    "C-'" => {
                        panes.push(Pane::new(
                            ttf_context
                                .load_font("data/LiberationSans-Regular.ttf", 16)
                                .unwrap(),
                            PaneType::Buffer,
                            0,
                        ));
                        arrange(&canvas, &mut panes);
                        pane_idx += 1;
                    }
                    "C-B" => panes[pane_idx].buffer_id = next(&buffers, panes[pane_idx].buffer_id),
                    "C-S-B" => {
                        panes[pane_idx].buffer_id = prev(&buffers, panes[pane_idx].buffer_id)
                    }
                    "C-J" => pane_idx = next(&panes, pane_idx),
                    "C-K" => pane_idx = prev(&panes, pane_idx),
                    "C-Q" => break 'mainloop,
                    "C-O" => {
                        let mut buffer = Buffer::new();
                        let mut pane = Pane::new(
                            ttf_context
                                .load_font("data/LiberationSans-Regular.ttf", 16)
                                .unwrap(),
                            PaneType::FileManager,
                            0,
                        );
                        let current_dir = env::current_dir().unwrap();
                        fm.update(&mut pane, &mut buffer, current_dir.to_str().unwrap());
                        pane.buffer_id = buffers.len();
                        pane_idx = panes.len();
                        buffers.push(buffer);
                        panes.push(pane);
                        arrange(&canvas, &mut panes);
                    }
                    "C-W" => {
                        if panes.len() > 1 {
                            panes.remove(pane_idx);
                            pane_idx = prev(&panes, pane_idx);
                            arrange(&canvas, &mut panes);
                        }
                    }
                    _ => {
                        let pane = &mut panes[pane_idx];
                        let buffer = &mut buffers[pane.buffer_id];
                        match pane.pane_type {
                            PaneType::Buffer => {
                                if handle_local_keystroke(pane, buffer, kstr) {
                                    break 'mainloop;
                                }
                            }
                            PaneType::FileManager => {
                                fm.handle_key(pane, buffer, kstr);
                            }
                        }
                    }
                }
            } else {
                let pane = &mut panes[pane_idx];
                let buffer = &mut buffers[pane.buffer_id];
                match event {
                    Event::Quit { .. } => break 'mainloop,
                    Event::KeyUp { keymod, .. } => {
                        if keymod.contains(Mod::RCTRLMOD) || keymod.contains(Mod::LCTRLMOD) {
                            ctrl_pressed = false;
                        }
                        if keymod.contains(Mod::RALTMOD) || keymod.contains(Mod::LALTMOD) {
                            alt_pressed = false;
                        }
                    }
                    Event::Window { win_event, .. } => {
                        if let WindowEvent::Resized(w, h) = win_event {
                            pane.w = max(0, w - 40) as u32;
                            pane.h = max(0, h - 40) as u32;
                        }
                    }
                    Event::TextInput { text, .. } => match pane.pane_type {
                        PaneType::Buffer => {
                            if !ctrl_pressed && !alt_pressed {
                                insert_text(pane, buffer, text);
                            }
                        }
                        PaneType::FileManager => {
                            fm.current_search.push_str(&text);
                            buffer.name = fm.current_search.clone();
                            for (i, line) in buffer.contents.iter().enumerate() {
                                if line.starts_with(&fm.current_search) {
                                    pane.select_line(i, &buffer);
                                }
                            }
                        }
                    },
                    Event::MouseButtonDown { x, y, .. } => {
                        set_selection_from_screen(pane, buffer, x, y, false)
                    }
                    Event::MouseMotion {
                        mousestate, x, y, ..
                    } => {
                        if mousestate.is_mouse_button_pressed(MouseButton::Left) {
                            set_selection_from_screen(pane, buffer, x, y, true);
                        }
                    }
                    Event::MouseWheel { y, .. } => scroll(pane, buffer, y),
                    Event::KeyDown { .. } => {}
                    _ => {}
                }
            }
        }

        // if is_dirty {
        draw(&mut panes, &mut buffers, pane_idx, &mut canvas);
        canvas.present();
        // }

        sleep(Duration::from_millis(5));
    }
}
