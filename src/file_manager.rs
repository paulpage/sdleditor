use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use crate::buffer::Buffer;
use crate::pane::{Pane, PaneType};

pub struct FileManagerEntry {
    name: String,
    is_dir: bool,
}

pub struct FileManager {
    pub current_search: String,
    pub entries: Vec<FileManagerEntry>,
    pub current_dir: PathBuf,
}

impl FileManager {
    pub fn new() -> Self {
        Self {
            current_dir: env::current_dir().unwrap(),
            current_search: String::new(),
            entries: Vec::new(),
        }
    }

    pub fn update(&mut self, buffer: &mut Buffer) {
        self.entries.clear();
        self.current_search.clear();
        buffer.clear();
        self.entries.push(FileManagerEntry {
            name: "..".to_string(),
            is_dir: true,
        });
        let paths = fs::read_dir(&self.current_dir).unwrap();
        for path in paths {
            let path = path.unwrap();
            self.entries.push(FileManagerEntry {
                name: path.file_name().into_string().unwrap(),
                is_dir: path.file_type().unwrap().is_dir(),
            });
        }
        for entry in &self.entries {
            buffer.push_line(format!(
                "{}{}",
                &entry.name,
                if entry.is_dir { "/" } else { "" }
            ));
        }
        buffer.select_line(0);
    }

    pub fn handle_key(&mut self, mut pane: &mut Pane, mut buffer: &mut Buffer, kstr: &str) {
        match kstr {
            "backspace" => {
                if !self.current_search.is_empty() {
                    self.current_search.remove(self.current_search.len() - 1);
                    buffer.name = self.current_search.clone();
                }
            }
            "return" => {
                if self.entries[buffer.sel_y].is_dir {
                    let entry = &self.entries[buffer.sel_y].name;
                    if entry == ".." {
                        if let Some(path) = self.current_dir.parent() {
                            self.current_dir = path.to_path_buf();
                        }
                    } else {
                        self.current_dir =
                            Path::join(&self.current_dir, &self.entries[buffer.sel_y].name);
                    }
                    self.update(buffer);
                } else {
                    let path = Path::join(&env::current_dir().unwrap(), &self.current_dir);
                    let f = Path::join(&path, self.entries[buffer.sel_y].name.clone());
                    *buffer = Buffer::from_path(f.display().to_string());
                    pane.pane_type = PaneType::Buffer;
                    buffer.cursor_x = 0;
                    buffer.cursor_y = 0;
                    buffer.set_selection(false);
                }
            }
            "down" => {
                buffer.cursor_down(1, false);
                buffer.select_line(buffer.cursor_y);
            }
            "up" => {
                buffer.cursor_up(1, false);
                buffer.select_line(buffer.cursor_y);
            }
            "escape" => {
                self.current_search.clear();
                buffer.name = self.current_search.clone();
            }
            _ => {}
        }
    }
}
