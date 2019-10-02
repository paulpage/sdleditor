use std::fs::OpenOptions;
use std::io::{self, BufReader, BufRead, BufWriter, Write};

pub struct Buffer {
    pub name: String,
    pub contents: Vec<String>,
    pub is_dirty: bool,
}

enum ActionType {
    Insert,
    Delete,
}

struct Action {
    action_type: ActionType,
    text: String,
    x: usize,
    y: usize,
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

    pub fn print(&self) {
        let f = io::stdout();
        let mut f = BufWriter::new(f.lock());
        for line in &self.contents {
            writeln!(&mut f, "{}", line).unwrap();
        }
        f.flush().unwrap();
    }

    pub fn insert(&mut self, x: usize, y: usize, contents: &str) {
        self.contents[y].insert_str(x, contents);
    }

    // TODO: probably doesn't work
    pub fn delete(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        if y1 == y2 {
            self.contents[y1].replace_range(x1..x2, "");
        } else if y1 > y2 {
            self.contents[y2].replace_range(..x2, "");
            for _ in y1+1..y2 {
                self.contents.remove(y1+1);
            }
            self.contents[y1].replace_range(x1.., "");
        }
    }
}
