use std::fs::OpenOptions;
use std::io::{BufReader, BufRead, BufWriter, Write};

pub struct Buffer {
    pub name: String,
    pub contents: Vec<String>,
    pub is_dirty: bool,
}

impl Buffer {

    pub fn new() -> Self {
        let mut buffer = Self {
            contents: Vec::new(),
            name: "UNNAMED".to_string(),
            is_dirty: false,
        };
        buffer.contents.push(String::new());
        buffer
    }

    pub fn from_path(path: String) -> Self {
        let mut buffer = Self {
            contents: Vec::new(),
            name: path.clone(),
            is_dirty: false,
        };
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&path)
            .unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            buffer.contents.push(line.unwrap());
        }
        if buffer.contents.len() == 0 {
            buffer.contents.push(String::new());
        }
        buffer
    }

    pub fn save(&mut self) {
        let f = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.name)
            .unwrap();

        let mut f = BufWriter::new(f);
        for line in &self.contents {
            write!(&mut f, "{}\n", line).unwrap();
        }
        f.flush().unwrap();
        self.is_dirty = false;
    }

}
