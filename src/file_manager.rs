use std::fs;
use std::env;

use crate::buffer::Buffer;
use crate::pane::{Pane, PaneType};

pub struct FileManagerEntry {
    name: String,
    is_dir: bool,
}

pub struct FileManager {
    pub current_search: String,
    pub entries: Vec<FileManagerEntry>,
}

impl FileManager {

    pub fn new() -> Self {
        Self {
            current_search: String::new(),
            entries: Vec::new(),
        }
    }

    pub fn update(&mut self, pane: &mut Pane, buffer: &mut Buffer, path: &str) {
        self.entries.clear();
        self.current_search.clear();
        buffer.contents.clear();
        self.entries.push(FileManagerEntry {
            name: "..".to_string(),
            is_dir: true,
        });
        let paths = fs::read_dir(&path).unwrap();
        for path in paths {
            let path = path.unwrap();
            self.entries.push(FileManagerEntry {
                name: path.file_name().into_string().unwrap(),
                is_dir: path.file_type().unwrap().is_dir(),
            });
        }
        for entry in &self.entries {
            buffer.contents.push(
                format!(
                    "{}{}",
                    &entry.name,
                    if entry.is_dir { "/" } else { "" }));
        }
        pane.select_line(0, &buffer);
    }

    pub fn handle_key(&mut self, mut pane: &mut Pane, mut buffer: &mut Buffer, kstr: &str) {
        match kstr {
            "Backspace" => {
                if self.current_search.len() > 0 {
                    self.current_search.remove(self.current_search.len() - 1);
                    buffer.name = self.current_search.clone();
                }
            }
            "Return" => {
                if self.entries[pane.sel_y].is_dir {
                    env::set_current_dir(&self.entries[pane.sel_y].name).unwrap();
                    let current_dir = env::current_dir().unwrap();
                    self.update(&mut pane, &mut buffer, current_dir.to_str().unwrap());
                } else {
                    *buffer = Buffer::from_path(self.entries[pane.sel_y].name.clone());
                    pane.pane_type = PaneType::Buffer;
                    pane.cursor_x = 0;
                    pane.cursor_y = 0;
                    pane.set_selection(false);
                }
            }
            _ => {}
        }
    }
}
