use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

use pgfx::app::App;
use pgfx::types::{Color, Rect};

mod pane;
use pane::{Pane, PaneType};

mod buffer;
use buffer::Buffer;

mod file_manager;
use file_manager::FileManager;

fn select_font() -> Option<PathBuf> {
    return Some(PathBuf::from("fonts/monospace.ttf"));
}

fn draw(
    app: &mut App,
    panes: &mut Vec<Pane>,
    buffers: &mut Vec<Buffer>,
    pane_idx: usize,
) {
    app.clear(Color::new(0, 0, 0));
    for (j, pane) in &mut panes.iter_mut().enumerate() {
        pane.draw(app, &buffers[pane.buffer_id], j == pane_idx);
    }
}

fn next(idx: usize, len: usize) -> usize {
    (idx + 1) % len
}

fn prev(idx: usize, len: usize) -> usize {
    (idx + len - 1) % len
}

fn arrange(app: &App, panes: &mut Vec<Pane>) {
    let w = app.window_width;
    let h = app.window_height;

    let padding = 5.0;
    let pane_width = (w / panes.len() as f32).floor();
    let pane_height = h;
    let mut x = 0.0;
    let y = 0.0;
    for mut pane in &mut panes.iter_mut() {
        pane.rect = Rect {
            x: x + padding,
            y: y + padding,
            width: f32::max(0.0, pane_width - (padding * 2.0)),
            height: f32::max(0.0, pane_height - (padding * 2.0)),
        };
        x += pane_width;
    }
}

fn main() {
    let path = match select_font() {
        Some(p) => p,
        None => PathBuf::new(),
    };
    let mut app = App::new("Sdleditor", path.to_str().unwrap(), 32.0);

    let mut buffers = Vec::new();
    let mut panes = Vec::new();
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
        PaneType::Buffer,
        0,
        app.font_size,
    ));
    arrange(&app, &mut panes);

    let mut fm = FileManager::new();
    let mut needs_redraw = false;

    while !app.should_quit() {
        needs_redraw = app.has_events;

        let mut should_quit = false;
        for key in &app.keys_pressed {
            let kstr = app.get_key_string(key);
            match kstr.as_str() {
                "c-'" => {
                    panes.push(Pane::new(
                        PaneType::Buffer,
                        panes[pane_idx].buffer_id,
                        app.font_size
                    ));
                    arrange(&app, &mut panes);
                    pane_idx += 1;
                }
                "c-b" => panes[pane_idx].buffer_id = next(panes[pane_idx].buffer_id, buffers.len()),
                "c-s-b" => {
                    panes[pane_idx].buffer_id = prev(panes[pane_idx].buffer_id, buffers.len())
                }
                "c-j" => pane_idx = next(pane_idx, panes.len()),
                "c-k" => pane_idx = prev(pane_idx, panes.len()),
                "c-q" => should_quit = true,
                "c-o" => {
                    let mut buffer = Buffer::new();
                    fm.current_dir = env::current_dir().unwrap();
                    fm.update(&mut buffer);
                    panes[pane_idx].buffer_id = buffers.len();
                    panes[pane_idx].pane_type = PaneType::FileManager;
                    panes[pane_idx].scroll_offset = 0;
                    buffers.push(buffer);
                }
                "c-w" => {
                    if panes.len() > 1 {
                        panes.remove(pane_idx);
                        pane_idx = prev(pane_idx, panes.len());
                        arrange(&app, &mut panes);
                    }
                }
                _ => {
                    let mut buf = &mut buffers[panes[pane_idx].buffer_id];
                    match panes[pane_idx].pane_type {
                        PaneType::Buffer => {
                            if panes[pane_idx].handle_keystroke(&mut buf, kstr.as_str()) {
                                should_quit = true;
                            }
                        }
                        PaneType::FileManager => {
                            fm.handle_key(&mut panes[pane_idx], &mut buf, kstr.as_str());
                        }
                    }
                }
            }
        }
        if should_quit {
            app.quit();
        }

        if app.window_size_changed {
            panes[pane_idx].rect.width = f32::max(0.0, app.window_width - 40.0);
            panes[pane_idx].rect.height = f32::max(0.0, app.window_height - 40.0);
            arrange(&app, &mut panes);
        }

        for text in &app.text_entered {
            let mut buf = &mut buffers[panes[pane_idx].buffer_id];
            match panes[pane_idx].pane_type {
                PaneType::Buffer => {
                    buf.action_insert_text(text.to_string());
                }
                PaneType::FileManager => {
                    fm.current_search.push_str(&text);
                    buf.name = fm.current_search.clone();
                    let mut selection = buf.cursor_y;
                    'searchloop: for (i, line) in
                        buf.contents[buf.cursor_y..].iter().enumerate()
                    {
                        if line.starts_with(&fm.current_search) {
                            selection = i + buf.cursor_y;
                            break 'searchloop;
                        }
                    }
                    buf.select_line(selection);
                }
            }
        }

        if app.mouse_left_pressed {
            let mut buf = &mut buffers[panes[pane_idx].buffer_id];
            panes[pane_idx].set_selection_from_screen(&mut buf, false);
            if app.mouse_left_clicks > 1 {
                let (x, y) = buf.prev_word(buf.cursor_x, buf.cursor_y);
                buf.sel_x = x;
                buf.sel_y = y;
                let (x, y) = buf.next_word(buf.cursor_x, buf.cursor_y);
                buf.cursor_x = x;
                buf.cursor_y = y;
            }
        }
        if app.mouse_left_down {
            let mut buf = &mut buffers[panes[pane_idx].buffer_id];
            panes[pane_idx].set_selection_from_screen(&mut buf, true);
        }
        if app.scroll.y != 0.0 {
            let mut buf = &mut buffers[panes[pane_idx].buffer_id];
            panes[pane_idx].scroll(&mut buf, app.scroll.y * -5.0);
        }

        for pane in &panes {
            if pane.scroll_lag != 0.0 {
                needs_redraw = true;
            }
        }

        if needs_redraw {
            draw(&mut app, &mut panes, &mut buffers, pane_idx);
            app.present();
        }

        sleep(Duration::from_millis(5));
    }
}
