extern crate clipboard;
extern crate sdl2;

use std::env;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::thread;
use std::sync::mpsc::channel;
use std::collections::HashMap;
// use std::time::Instant;

use sdl2::event::{Event, WindowEvent};
use sdl2::keyboard::{Keycode, Mod};
use sdl2::mouse::MouseButton;
use sdl2::render::WindowCanvas;

mod pane;
use pane::{Pane, TextCell};

mod neovim_connector;
use neovim_connector::{NvimEvent, ClientEvent, NvimMode};

struct InputState {
    alt_down: bool,
    ctrl_down: bool,
    shift_down: bool,
    mouse_row: i32,
    mouse_col: i32,
    mouse_button: String,
    mode: NvimMode,
    num_rows: i64,
    num_cols: i64,
}

fn select_font() -> Option<PathBuf> {
    match font_kit::source::SystemSource::new().select_best_match(
        &[font_kit::family_name::FamilyName::Monospace],
        &font_kit::properties::Properties::new(),
    ) {
        Ok(font_kit::handle::Handle::Path { path, .. }) => Some(path),
        _ => None,
    }
}

fn main() {

    let (server_sender, server_receiver) = channel();
    let (client_sender, client_receiver) = channel();
    thread::spawn(move|| {
        neovim_connector::start(server_sender, client_receiver, env::args());
    });

    let path = match select_font() {
        Some(p) => p,
        None => PathBuf::new(),
    };

    let mut input = InputState {
        alt_down: false,
        ctrl_down: false,
        shift_down: false,
        mouse_row: 0,
        mouse_col: 0,
        mouse_button: String::new(),
        mode: NvimMode::Normal,
        num_rows: 0,
        num_cols: 0,
    };

    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().unwrap();
    let window = video_subsys
        .window("Neovim", 800, 600)
        .position_centered()
        .resizable()
        .maximized()
        .opengl()
        .build()
        .unwrap();
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();

    let font = ttf_context.load_font(&path, 16).unwrap();
    let col_width = font.size_of_char('W').unwrap().0 as i32;
    let row_height = font.height();
    let (window_w, window_h) = canvas.window().size();
    input.num_cols = window_w as i64 / col_width as i64;
    input.num_rows = window_h as i64 / row_height as i64;
    client_sender.send(ClientEvent::WindowResize {
        cols: input.num_cols,
        rows: input.num_rows,
    }).unwrap();

    let mut text = vec![vec![TextCell { text: " ".to_string(), hl_id: 0 }; input.num_cols as usize]; input.num_rows as usize];

    let mut pane = Pane::new(
        ttf_context.load_font(&path, 16).unwrap()
    );

    let mut highlight_table = HashMap::new();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                Event::KeyDown { keycode: Some(kc), keymod, .. } => {
                    input.shift_down = keymod.contains(Mod::LSHIFTMOD) || keymod.contains(Mod::RSHIFTMOD);
                    input.ctrl_down = keymod.contains(Mod::LCTRLMOD) || keymod.contains(Mod::RCTRLMOD);
                    input.alt_down = keymod.contains(Mod::LALTMOD) || keymod.contains(Mod::RALTMOD);

                    // These keys should only be send with a modifier, otherwise they're handled by
                    // the text input event.
                    let mut key_to_send = match kc {
                        Keycode::Space => "Space",
                        // Keycode::Exclaim => "",
                        // Keycode::Quotedbl => "",
                        // Keycode::Hash => "",
                        // Keycode::Dollar => "",
                        // Keycode::Percent => "",
                        // Keycode::Ampersand => "",
                        // Keycode::Quote => "",
                        // Keycode::LeftParen => "",
                        // Keycode::RightParen => "",
                        // Keycode::Asterisk => "",
                        // Keycode::Plus => "",
                        Keycode::Comma => ",",
                        Keycode::Minus => "-",
                        Keycode::Period => ".",
                        Keycode::Slash => "/",
                        Keycode::Num0 => "0",
                        Keycode::Num1 => "1",
                        Keycode::Num2 => "2",
                        Keycode::Num3 => "3",
                        Keycode::Num4 => "4",
                        Keycode::Num5 => "5",
                        Keycode::Num6 => "6",
                        Keycode::Num7 => "7",
                        Keycode::Num8 => "8",
                        Keycode::Num9 => "9",
                        Keycode::Colon => "",
                        Keycode::Semicolon => ";",
                        // Keycode::Less => "",
                        Keycode::Equals => "=",
                        // Keycode::Greater => "",
                        // Keycode::Question => "",
                        // Keycode::At => "",
                        Keycode::LeftBracket => "[",
                        Keycode::Backslash => "\\",
                        Keycode::RightBracket => "]",
                        // Keycode::Caret => "",
                        // Keycode::Underscore => "",
                        Keycode::Backquote => "`",
                        Keycode::A => "a",
                        Keycode::B => "b",
                        Keycode::C => "c",
                        Keycode::D => "d",
                        Keycode::E => "e",
                        Keycode::F => "f",
                        Keycode::G => "g",
                        Keycode::H => "h",
                        Keycode::I => "i",
                        Keycode::J => "j",
                        Keycode::K => "k",
                        Keycode::L => "l",
                        Keycode::M => "m",
                        Keycode::N => "n",
                        Keycode::O => "o",
                        Keycode::P => "p",
                        Keycode::Q => "q",
                        Keycode::R => "r",
                        Keycode::S => "s",
                        Keycode::T => "t",
                        Keycode::U => "u",
                        Keycode::V => "v",
                        Keycode::W => "w",
                        Keycode::X => "x",
                        Keycode::Y => "y",
                        Keycode::Z => "z",
                        Keycode::Caret => "^",
                        _ => "",
                    };
                    if key_to_send != "" && (input.ctrl_down || input.alt_down) {
                        if input.shift_down {
                            key_to_send = match key_to_send {
                                "0" => ")",
                                "1" => "!",
                                "2" => "@",
                                "3" => "#",
                                "4" => "$",
                                "5" => "%",
                                "6" => "^",
                                "7" => "&",
                                "8" => "*",
                                "9" => "(",
                                "," => "<",
                                "-" => "_",
                                "." => ">",
                                "/" => "?",
                                ";" => ":",
                                "=" => "+",
                                "[" => "{",
                                "\\" => "|",
                                "]" => "}",
                                "`" => "~",
                                _ => key_to_send,
                            };
                        }
                        client_sender.send(ClientEvent::Text(
                            format!("<{}{}{}>",
                                if input.alt_down { "M-" } else { "" },
                                if input.ctrl_down { "C-" } else { "" },
                                key_to_send,
                            )
                        )).unwrap();
                    }

                    // These keys should always be sent, regardless of modifiers.
                    let key_to_send = match kc {
                        Keycode::Backspace => "BS",
                        Keycode::Tab => "Tab",
                        Keycode::Return => "CR",
                        Keycode::Escape => "Esc",
                        Keycode::Delete => "Del",
                        Keycode::CapsLock => "",
                        Keycode::F1 => "F1",
                        Keycode::F2 => "F2",
                        Keycode::F3 => "F3",
                        Keycode::F4 => "F4",
                        Keycode::F5 => "F5",
                        Keycode::F6 => "F6",
                        Keycode::F7 => "F7",
                        Keycode::F8 => "F8",
                        Keycode::F9 => "F9",
                        Keycode::F10 => "F10",
                        Keycode::F11 => "F11",
                        Keycode::F12 => "F12",
                        Keycode::PrintScreen => "",
                        Keycode::ScrollLock => "",
                        Keycode::Pause => "",
                        Keycode::Insert => "Insert",
                        Keycode::Home => "Home",
                        Keycode::PageUp => "PageUp",
                        Keycode::End => "End",
                        Keycode::PageDown => "PageDown",
                        Keycode::Right => "Right",
                        Keycode::Left => "Left",
                        Keycode::Down => "Down",
                        Keycode::Up => "Up",
                        Keycode::NumLockClear => "",
                        Keycode::KpDivide => "kDivide",
                        Keycode::KpMultiply => "kMultiply",
                        Keycode::KpMinus => "kMinus",
                        Keycode::KpPlus => "kPlus",
                        Keycode::KpEnter => "kEnter",
                        Keycode::Kp1 => "k1",
                        Keycode::Kp2 => "k2",
                        Keycode::Kp3 => "k3",
                        Keycode::Kp4 => "k4",
                        Keycode::Kp5 => "k5",
                        Keycode::Kp6 => "k6",
                        Keycode::Kp7 => "k7",
                        Keycode::Kp8 => "k8",
                        Keycode::Kp9 => "k9",
                        Keycode::Kp0 => "k0",
                        Keycode::KpPeriod => "kPoint",
                        Keycode::Execute => "",
                        Keycode::Help => "Help",
                        Keycode::Undo => "Undo",
                        _ => "",
                    };

                    if key_to_send != "" {
                        client_sender.send(ClientEvent::Text(
                            format!("<{}{}{}{}>",
                                if input.alt_down { "M-" } else { "" },
                                if input.ctrl_down { "C-" } else { "" },
                                if input.shift_down { "S-" } else { "" },
                                key_to_send,
                            )
                        )).unwrap();
                    }
                }
                Event::TextInput { text, .. } => {
                    client_sender.send(ClientEvent::Text(text)).unwrap();
                }
                Event::KeyUp { keymod, .. } => {
                    input.shift_down = keymod.contains(Mod::LSHIFTMOD | Mod::RSHIFTMOD);
                    input.ctrl_down = keymod.contains(Mod::LCTRLMOD | Mod::RCTRLMOD);
                    input.alt_down = keymod.contains(Mod::LALTMOD | Mod::RALTMOD);
                }
                Event::Window { win_event, .. } => {
                    if let WindowEvent::Resized(w, h) = win_event {
                        let num_cols = w as i64 / col_width as i64;
                        let num_rows = h as i64 / row_height as i64;
                        client_sender.send(ClientEvent::WindowResize {
                            cols: num_cols,
                            rows: num_rows,
                        }).unwrap();
                    }
                }
                Event::MouseButtonDown { x, y, clicks, mouse_btn, .. } => {
                    input.mouse_col = x / col_width;
                    input.mouse_row = y / row_height;
                    let button = match mouse_btn {
                        MouseButton::Left => "left",
                        MouseButton::Right => "right",
                        MouseButton::Middle => "middle",
                        _ => "",
                    };
                    input.mouse_button = button.clone().into();
                    if button != "" {
                        for _ in 0..clicks {
                            client_sender.send(ClientEvent::Mouse {
                                button: button.into(),
                                action: "press".into(),
                                modifier: "".into(), // TODO
                                grid: 0,
                                col: input.mouse_col.into(),
                                row: input.mouse_row.into(),
                            }).unwrap();
                        }
                    }
                }
                Event::MouseButtonUp { mouse_btn, .. } => {
                    let button = match mouse_btn {
                        MouseButton::Left => "left",
                        MouseButton::Right => "right",
                        MouseButton::Middle => "middle",
                        _ => "",
                    };
                    client_sender.send(ClientEvent::Mouse {
                        button: button.into(),
                        action: "release".into(),
                        modifier: "".into(), // TODO
                        grid: 0,
                        col: input.mouse_col.into(),
                        row: input.mouse_row.into(),
                    }).unwrap();
                    input.mouse_button = String::new();
                }
                Event::MouseMotion {
                    x, y, ..
                } => {
                    if input.mouse_button != "" {
                        input.mouse_col = x / col_width;
                        input.mouse_row = y / row_height;
                        client_sender.send(ClientEvent::Mouse {
                            button: input.mouse_button.clone(),
                            action: "drag".into(),
                            modifier: "".into(), // TODO
                            grid: 0,
                            col: input.mouse_col.into(),
                            row: input.mouse_row.into(),
                        }).unwrap();
                    }
                }
                Event::MouseWheel { x, y, .. } => {
                    let action = match (x, y) {
                        (0, 1) => "up",
                        (0, -1) => "down",
                        (1, 0) => "right",
                        (-1, 0) => "left",
                        _ => "",
                    };
                    client_sender.send(ClientEvent::Mouse {
                        button: "wheel".into(),
                        action: action.into(),
                        modifier: "".into(), // TODO
                        grid: 0,
                        col: input.mouse_col.into(),
                        row: input.mouse_row.into(),
                    }).unwrap();
                }
                _ => {}
            }
        }

        let has_received = match server_receiver.try_recv() {
            Ok(e) => {
                match e {
                    NvimEvent::GridLine(entries) => {
                        let mut last_hl_id = -1;
                        for entry in entries {
                            let row = entry.row as usize;
                            let mut col = entry.col as usize;
                            for cell in entry.cells {
                                let hl = if cell.highlight == -1 { last_hl_id } else { cell.highlight };
                                for _ in 0..cell.repeat {
                                    text[row][col] = TextCell { text: cell.text.clone(), hl_id: hl };
                                    col += 1;
                                }
                                last_hl_id = if cell.highlight == -1 { last_hl_id } else { cell.highlight };
                            }
                        }
                        false
                    }
                    NvimEvent::Flush => true,
                    NvimEvent::GridCursorGoto(_grid, row, col) => {
                        pane.cursor_row = row as i32;
                        pane.cursor_col = col as i32;
                        false
                    }
                    NvimEvent::GridClear(_) => {
                        text = vec![vec![TextCell { text: " ".to_string(), hl_id: 0 }; input.num_cols as usize]; input.num_rows as usize];
                        false
                    }
                    NvimEvent::GridScroll(e) => {
                        if e.rows > 0 {
                            for y in e.top..e.bot {
                                for x in e.left..e.right {
                                    let y_idx = y - e.rows;
                                    if y_idx >= 0 {
                                        text[y_idx as usize][x as usize] = text[y as usize][x as usize].clone();
                                        text[y as usize][x as usize] = TextCell {text: " ".to_string(), hl_id: 0};
                                    }
                                }
                            }
                        } else {
                            for y in (e.top..e.bot-1).rev() {
                                for x in (e.left..e.right).rev() {
                                    let y_idx = (y - e.rows) as usize;
                                    if y_idx < text.len() {
                                        text[y_idx as usize][x as usize] = text[y as usize][x as usize].clone();
                                        text[y as usize][x as usize] = TextCell { text: " ".to_string(), hl_id: 0};
                                    }
                                }
                            }
                        }
                        false
                    }
                    NvimEvent::DefaultColorsSet { fg, bg, special } => {
                        pane.set_colors(fg, bg, special);
                        false
                    }
                    NvimEvent::Close => {
                        break 'mainloop;
                    }
                    NvimEvent::ModeChange(mode) => {
                        input.mode = mode;
                        false
                    }
                    NvimEvent::ModeInfoSet(mode_info) => {
                        false
                    }
                    NvimEvent::HighlightAttrDefine{ id, hl } => {
                        highlight_table.insert(id, hl);
                        false
                    }
                    NvimEvent::GridResize { cols, rows, .. } => {
                        input.num_cols = cols;
                        input.num_rows = rows;
                        text = vec![vec![TextCell { text: " ".to_string(), hl_id: 0 }; input.num_cols as usize]; input.num_rows as usize];
                        let (w, h) = canvas.window().size();
                        pane.w = w as u32;
                        pane.h = h as u32;
                        // client_sender.send(ClientEvent::WindowResize {
                        //     cols: input.num_cols,
                        //     rows: input.num_rows,
                        // }).unwrap();
                        true
                    }
                }
            }
            Err(_) => {
                // sleep(Duration::from_millis(1));
                false
            }
        };

        if has_received {
            pane.draw(&mut canvas, &text, &highlight_table);
            canvas.present();
        }
        sleep(Duration::from_millis(1));

    }
}
