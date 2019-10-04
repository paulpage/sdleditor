use std::fs::OpenOptions;
use std::io::{self, BufReader, BufRead, BufWriter, Write};

pub struct Buffer {
    pub name: String,
    pub contents: Vec<String>,
    pub is_dirty: bool,
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
}

#[derive(Clone)]
pub enum ActionType {
    Insert,
    Delete,
}

#[derive(Clone)]
pub struct Action {
    action_type: ActionType,
    text: String,
    x1: usize,
    y1: usize,
    x2: usize,
    y2: usize,
}

impl Buffer {

    pub fn new() -> Self {
        let mut buffer = Self {
            contents: Vec::new(),
            name: "UNNAMED".to_string(),
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        buffer.contents.push(String::new());
        buffer
    }

    pub fn from_path(path: String) -> Self {
        let mut buffer = Self {
            contents: Vec::new(),
            name: path.clone(),
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
        };
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(&path)
            .unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            buffer.contents.push(line.unwrap());
        }
        if buffer.contents.is_empty() {
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
            writeln!(&mut f, "{}", line).unwrap();
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

    pub fn delete_text(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let text = self.do_delete(x1, y1, x2, y2);
        self.undo_stack.push(Action {
            action_type: ActionType::Delete,
            text,
            x1,
            y1,
            x2,
            y2,
        });
    }

    pub fn insert_text(&mut self, x: usize, y: usize, text: String) {
        let (x2, y2) = self.do_insert(x, y, text.clone());
        self.undo_stack.push(Action {
            action_type: ActionType::Insert,
            text,
            x1: x,
            y1: y,
            x2,
            y2,
        });
    }

    fn do_delete(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) -> String {
        let mut undo_buffer = Vec::new();
        let pre = self.contents[y1][..x1].to_string();
        let npre = self.contents[y1][x1..].to_string();
        let post = self.contents[y2][x2..].to_string();
        let npost = self.contents[y2][..x2].to_string();
        for _ in y1..=y2 {
            undo_buffer.push(self.contents.remove(y1));
        }
        let end = undo_buffer.len() - 1;
        undo_buffer[0] = npre;
        undo_buffer[end] = npost;
        println!("{}", undo_buffer.join("\n"));
        self.contents.insert(y1, format!("{}{}", pre, post));
        undo_buffer.join("\n")
    }

    fn do_insert(&mut self, x: usize, y: usize, text: String) -> (usize, usize) {
        let l = self.contents.remove(y);
        let (start, end) = l.split_at(x);
        self.contents.insert(y, start.to_string());
        let mut x = x;
        let mut y = y;
        for c in text.chars() {
            if c == '\n' {
                y += 1;
                x = 0;
                self.contents.insert(y, String::new());
            } else {
                self.contents[y].push(c);
                x += 1;
            }
        }
        self.contents[y].push_str(end);
        (x, y)
    }
 
    // TODO: Still acts funky, try typing "Hello, World!" into an empty document,
    // deleting some of the middle, then undoing
    pub fn undo(&mut self) {
        if let Some(a) = self.undo_stack.pop() {
            match a.action_type {
                ActionType::Delete => {
                    let (x2, y2) = self.do_insert(a.x1, a.y1, a.text.clone());
                    self.redo_stack.push(Action {
                        action_type: ActionType::Delete,
                        text: a.text,
                        x1: a.x1,
                        y1: a.y1,
                        x2,
                        y2,
                    });
                }
                ActionType::Insert => {
                    self.do_delete(a.x1, a.y1, a.x2, a.y2);
                    self.redo_stack.push(Action {
                        action_type: ActionType::Insert,
                        text: a.text,
                        x1: a.x1,
                        y1: a.y2,
                        x2: a.x2,
                        y2: a.y2,
                    });
                }
            }
        }
    }

    pub fn redo(&mut self) {
        if let Some(a) = self.redo_stack.pop() {
            match a.action_type {
                ActionType::Delete => {
                    self.delete_text(a.x1, a.y1, a.x2, a.y2);
                }
                ActionType::Insert => {
                    self.insert_text(a.x1, a.y1, a.text);
                }
            }
        }
    }

    pub fn next_char(&self, x: usize, y: usize) -> (usize, usize) {
        if x < self.contents[y].len() {
            return (x + 1, y);
        } else if y < self.contents.len() {
            return (0, y + 1);
        }
        (x, y)
    }

    pub fn prev_char(&self, x: usize, y: usize) -> (usize, usize) {
        if x > 0 {
            return (x - 1, y);
        } else if y > 0 {
            return (self.contents[y - 1].len(), y - 1);
        }
        (x, y)
    }
}
