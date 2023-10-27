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
    Some(PathBuf::from("fonts/monospace.ttf"))
}

fn next(idx: usize, len: usize) -> usize {
    (idx + 1) % len
}

fn prev(idx: usize, len: usize) -> usize {
    (idx + len - 1) % len
}

struct Editor {
    fm: FileManager,

    panes: Vec<Pane>,
    buffers: Vec<Buffer>,
    pane_idx: usize,

    window_width: f32,
    window_height: f32,
    font_size: f32,
    should_quit: bool,
}

impl Editor {
    fn draw(&mut self, app: &mut App) {
        app.clear(Color::new(0, 0, 0));
        for (j, pane) in &mut self.panes.iter_mut().enumerate() {
            pane.draw(app, &self.buffers[pane.buffer_id], j == self.pane_idx);
        }
    }

    fn arrange(&mut self) {
        let w = self.window_width;
        let h = self.window_height;

        let padding = 5.0;
        let pane_width = (w / self.panes.len() as f32).floor();
        let pane_height = h;
        let mut x = 0.0;
        let y = 0.0;
        for mut pane in &mut self.panes.iter_mut() {
            pane.rect = Rect {
                x: x + padding,
                y: y + padding,
                width: f32::max(0.0, pane_width - (padding * 2.0)),
                height: f32::max(0.0, pane_height - (padding * 2.0)),
            };
            x += pane_width;
        }
    }

    // Commands
    //========================================
    fn select_next_pane(&mut self) {
        self.pane_idx = next(self.pane_idx, self.panes.len());
    }

    fn select_prev_pane(&mut self) {
        self.pane_idx = prev(self.pane_idx, self.panes.len());
    }

    fn select_next_buffer(&mut self) {
        self.panes[self.pane_idx].buffer_id = next(self.panes[self.pane_idx].buffer_id, self.buffers.len());
    }

    fn select_prev_buffer(&mut self) {
        self.panes[self.pane_idx].buffer_id = prev(self.panes[self.pane_idx].buffer_id, self.buffers.len());
    }

    fn add_pane(&mut self) {
        let buffer_id = if !self.panes.is_empty() {
            self.panes[self.pane_idx].buffer_id
        } else {
            0
        };
        self.panes.push(Pane::new(
            PaneType::Buffer,
            buffer_id,
            self.font_size,
        ));
        self.arrange();
        self.pane_idx = self.panes.len() - 1;
    }

    fn close_pane(&mut self) {
        if self.panes.len() > 1 {
            self.panes.remove(self.pane_idx);
            self.select_prev_pane();
            self.arrange();
        }
    }

    fn open_file_dialog(&mut self) {
        let mut buffer = Buffer::new();
        self.fm.current_dir = env::current_dir().unwrap();
        self.fm.update(&mut buffer);
        self.panes[self.pane_idx].buffer_id = self.buffers.len();
        self.panes[self.pane_idx].pane_type = PaneType::FileManager;
        self.panes[self.pane_idx].scroll_offset = 0.0;
        self.buffers.push(buffer);
    }

    fn new_file(&mut self) {
        let buffer = Buffer::new();
        self.panes[self.pane_idx].buffer_id = self.buffers.len();
        self.panes[self.pane_idx].pane_type = PaneType::FileManager;
        self.panes[self.pane_idx].scroll_offset = 0.0;
        self.buffers.push(buffer);
    }

    fn quit(&mut self) {
        self.should_quit = true;
    }

    //========================================

    // Utils
    
    fn open_file(&mut self, path: &str) {
        let buffer = Buffer::from_path(path.to_string());
        self.panes[self.pane_idx].buffer_id = self.buffers.len();
        self.panes[self.pane_idx].pane_type = PaneType::Buffer;
        self.panes[self.pane_idx].scroll_offset = 0.0;
        self.buffers.push(buffer);
    }

    //========================================

    fn new(app: &App) -> Self {
        let mut editor = Editor {
            fm: FileManager::new(),
            buffers: Vec::new(),
            panes: Vec::new(),
            pane_idx: 0,
            window_width: app.window_width,
            window_height: app.window_height,
            font_size: app.font_size,
            should_quit: false,
        };
        editor.add_pane();
        // editor.new_file();
        editor.open_file("src/main.rs");
        editor
    }

    fn run(&mut self, app: &mut App) {
        while !app.should_quit() {

            let mut needs_redraw = app.has_events;

            for key in &app.keys_pressed {
                let kstr = app.get_key_string(key);
                match kstr.as_str() {
                    "c-'" => self.add_pane(),
                    "c-w" => self.close_pane(),
                    "c-j" => self.select_next_pane(),
                    "c-k" => self.select_prev_pane(),
                    "c-b" => self.select_next_buffer(),
                    "c-s-b" => self.select_prev_buffer(),
                    "c-o" => self.open_file_dialog(),
                    "c-q" => self.quit(),
                    _ => {
                        let buf = &mut self.buffers[self.panes[self.pane_idx].buffer_id];
                        match self.panes[self.pane_idx].pane_type {
                            PaneType::Buffer => {
                                if self.panes[self.pane_idx].handle_keystroke(buf, kstr.as_str()) {
                                    self.quit();
                                }
                            }
                            PaneType::FileManager => {
                                self.fm.handle_key(&mut self.panes[self.pane_idx], buf, kstr.as_str());
                            }
                        }
                    }
                }
            }

            if self.should_quit {
                app.quit();
            }

            if app.window_size_changed {
                self.window_width = app.window_width;
                self.window_height = app.window_height;
                // self.panes[self.pane_idx].rect.width = f32::max(0.0, app.window_width - 40.0);
                // self.panes[self.pane_idx].rect.height = f32::max(0.0, app.window_height - 40.0);
                self.arrange();
            }

            for text in &app.text_entered {
                let mut buf = &mut self.buffers[self.panes[self.pane_idx].buffer_id];
                match self.panes[self.pane_idx].pane_type {
                    PaneType::Buffer => {
                        buf.action_insert_text(text.to_string());
                    }
                    PaneType::FileManager => {
                        self.fm.current_search.push_str(text);
                        buf.name = self.fm.current_search.clone();
                        let mut selection = buf.cursor_y;
                        'searchloop: for (i, line) in
                            buf.contents[buf.cursor_y..].iter().enumerate()
                        {
                            if line.starts_with(&self.fm.current_search) {
                                selection = i + buf.cursor_y;
                                break 'searchloop;
                            }
                        }
                        buf.select_line(selection);
                    }
                }
            }

            if app.mouse_left_pressed {
                let mut buf = &mut self.buffers[self.panes[self.pane_idx].buffer_id];
                self.panes[self.pane_idx].set_selection_from_screen(buf, false);
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
                let buf = &mut self.buffers[self.panes[self.pane_idx].buffer_id];
                self.panes[self.pane_idx].set_selection_from_screen(buf, true);
            }
            if app.scroll.y != 0.0 {
                self.panes[self.pane_idx].scroll(app.scroll.y / -2.0);
            }

            for pane in &self.panes {
                if pane.scroll_lag != 0.0 {
                    needs_redraw = true;
                }
            }

            if needs_redraw {
                self.draw(app);
                app.present();
            }

            sleep(Duration::from_millis(5));
        }
    }
}

fn main() {
    let path = match select_font() {
        Some(p) => p,
        None => PathBuf::new(),
    };

    let mut app = App::new("Sdleditor", path.to_str().unwrap(), 16.0);
    let mut editor = Editor::new(&app);
    editor.run(&mut app); 
}
