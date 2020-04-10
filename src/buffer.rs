use std::cmp::{max, min};
use std::fs::OpenOptions;
use std::io::{self, BufRead, BufReader, BufWriter, Write};

use clipboard::{ClipboardContext, ClipboardProvider};

use unicode_segmentation::UnicodeSegmentation;

pub struct Buffer {
    pub name: String,
    pub contents: Vec<String>,
    pub is_dirty: bool,
    pub undo_stack: Vec<Action>,
    pub redo_stack: Vec<Action>,
    pub cursor_x: usize,
    pub max_cursor_x: usize,
    pub cursor_y: usize,
    pub sel_x: usize,
    pub sel_y: usize,
}

#[derive(Clone)]
pub struct Action {
    deleted_text: Option<String>,
    inserted_text: Option<String>,
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
            cursor_x: 0,
            max_cursor_x: 0,
            cursor_y: 0,
            sel_x: 0,
            sel_y: 0,
        };
        buffer.push_line(String::new());
        buffer
    }

    pub fn from_path(path: String) -> Self {
        let mut buffer = Self {
            contents: Vec::new(),
            name: path.clone(),
            is_dirty: false,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            cursor_x: 0,
            max_cursor_x: 0,
            cursor_y: 0,
            sel_x: 0,
            sel_y: 0,
        };
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .read(true)
            .open(&path)
            .unwrap();
        let reader = BufReader::new(file);
        for line in reader.lines() {
            buffer.push_line(line.unwrap());
        }
        if buffer.is_empty() {
            buffer.push_line(String::new());
        }
        buffer
    }

    pub fn len(&self) -> usize {
        self.contents.len()
    }

    pub fn line_len(&self, y: usize) -> usize {
        self.contents[y].graphemes(true).count()
    }

    pub fn line_graphemes(&self, y: usize) -> Vec<&str> {
        self.contents[y].graphemes(true).collect::<Vec<&str>>()
    }

    pub fn clear(&mut self) {
        self.contents.clear();
    }

    pub fn is_empty(&self) -> bool {
        self.contents.is_empty()
    }

    pub fn push_line(&mut self, s: String) {
        self.contents.push(s);
    }

    pub fn insert_line(&mut self, y: usize, s: String) {
        self.contents.insert(y, s);
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
        self.is_dirty = true;
        self.undo_stack.push(Action {
            deleted_text: Some(text),
            inserted_text: None,
            x1,
            y1,
            x2,
            y2,
        });
    }

    pub fn action_insert_text(&mut self, text: String) {
        let (x1, y1, x2, y2) = self.get_selection();
        let (new_x, new_y) = self.replace_text(x1, y1, x2, y2, text);
        self.cursor_x = new_x;
        self.cursor_y = new_y;
        self.set_selection(false);
    }

    pub fn select_all(&mut self) {
        self.sel_y = 0;
        self.sel_x = 0;
        self.cursor_y = max(0, self.len() as i32 - 1) as usize;
        self.cursor_x = self.line_len(self.cursor_y);
        self.set_selection(true);
    }

    pub fn set_selection(&mut self, extend_selection: bool) {
        if !extend_selection {
            self.sel_x = self.cursor_x;
            self.sel_y = self.cursor_y;
        }
    }

    pub fn break_line(&mut self) {
        let mut g = self.line_graphemes(self.cursor_y);
        let first_half = g.drain(..self.cursor_x).collect::<Vec<&str>>().concat();
        let last_half = g.concat();
        self.contents[self.cursor_y] = first_half;
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.max_cursor_x = self.cursor_x;
        self.insert_line(self.cursor_y, last_half);
        self.set_selection(false);
    }

    pub fn remove_selection(&mut self) {
        let (x1, y1, x2, y2) = self.get_selection();
        if x1 == x2 && y1 == y2 {
            self.remove_char();
        } else {
            self.delete_text(x1, y1, x2, y2);
            self.cursor_x = x1;
            self.cursor_y = y1;
        }
        self.set_selection(false);
    }

    pub fn cursor_up(&mut self, num: usize, extend_selection: bool) {
        self.cursor_y = max(0, self.cursor_y as i32 - num as i32) as usize;
        self.cursor_x = max(
            min(self.cursor_x, self.line_len(self.cursor_y)),
            min(self.max_cursor_x, self.line_len(self.cursor_y)),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_down(&mut self, num: usize, extend_selection: bool) {
        self.cursor_y = min(self.len() - 1, self.cursor_y + num);
        self.cursor_x = max(
            min(self.cursor_x, self.line_len(self.cursor_y)),
            min(self.max_cursor_x, self.line_len(self.cursor_y)),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_left(&mut self, extend_selection: bool) {
        let (x, y) = self.prev_char(self.cursor_x, self.cursor_y);
        self.cursor_x = x;
        self.cursor_y = y;
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn cursor_right(&mut self, extend_selection: bool) {
        let (x, y) = self.next_char(self.cursor_x, self.cursor_y);
        self.cursor_x = x;
        self.cursor_y = y;
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn clipboard_paste(&mut self) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(s) = ctx.get_contents() {
            self.insert_text(self.cursor_x, self.cursor_y, s);
        }
    }

    pub fn clipboard_copy(&mut self) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let (x1, y1, x2, y2) = self.get_selection();
        let s = self.do_delete(x1, y1, x2, y2);
        self.do_insert(x1, y2, s.clone());
        ctx.set_contents(s).unwrap();
    }

    pub fn clipboard_cut(&mut self) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let (x1, y1, x2, y2) = self.get_selection();
        let s = self.do_delete(x1, y1, x2, y2);
        ctx.set_contents(s).unwrap();
    }

    pub fn select_line(&mut self, line: usize) {
        self.cursor_y = min(line, max(0, self.len() as i32 - 1) as usize);
        self.sel_y = self.cursor_y;
        self.cursor_x = self.line_len(self.sel_y);
        self.sel_x = 0;
    }

    // A selection is defined by the cursor position as one corner
    // and the selection position at the other. This function
    // returns those corners in order: (x1, y1, x2, y2)
    // where x1 and y1 are earlier in the buffer than x2 and y2
    pub fn get_selection(&self) -> (usize, usize, usize, usize) {
        if self.sel_y > self.cursor_y || self.sel_y == self.cursor_y && self.sel_x > self.cursor_x {
            return (self.cursor_x, self.cursor_y, self.sel_x, self.sel_y);
        }
        (self.sel_x, self.sel_y, self.cursor_x, self.cursor_y)
    }

    pub fn remove_char(&mut self) {
        let (x1, y1) = self.prev_char(self.cursor_x, self.cursor_y);
        self.delete_text(x1, y1, self.cursor_x, self.cursor_y);
        self.cursor_x = x1;
        self.cursor_y = y1;
    }

    pub fn insert_text(&mut self, x: usize, y: usize, text: String) -> (usize, usize) {
        let (x2, y2) = self.do_insert(x, y, text.clone());
        self.is_dirty = true;
        self.undo_stack.push(Action {
            deleted_text: None,
            inserted_text: Some(text),
            x1: x,
            y1: y,
            x2,
            y2,
        });
        (x2, y2)
    }

    pub fn replace_text(
        &mut self,
        x1: usize,
        y1: usize,
        x2: usize,
        y2: usize,
        text: String,
    ) -> (usize, usize) {
        let deleted_text = self.do_delete(x1, y1, x2, y2);
        let (x2, y2) = self.do_insert(x1, y1, text.clone());
        self.is_dirty = true;
        self.undo_stack.push(Action {
            deleted_text: Some(deleted_text),
            inserted_text: Some(text),
            x1,
            y1,
            x2,
            y2,
        });
        (x2, y2)
    }

    pub fn do_delete(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) -> String {
        let mut undo_buffer = Vec::new();

        let mut g1 = self.line_graphemes(y1);
        let g2 = self.line_graphemes(y2);
        if y1 == y2 {
            undo_buffer.push(g1.drain(x1..x2).collect::<Vec<&str>>().concat());
            self.contents[y1] = g1.concat();
        } else {
            let pre = g1[..x1].concat();
            let npre = g1[x1..].concat();
            let post = g2[x2..].concat();
            let npost = g2[..x2].concat();
            for _ in y1..=y2 {
                undo_buffer.push(self.contents.remove(y1));
            }
            let end = undo_buffer.len() - 1;
            undo_buffer[0] = npre;
            undo_buffer[end] = npost;
            self.insert_line(y1, format!("{}{}", pre, post));
        }
        undo_buffer.join("\n")
    }

    pub fn do_insert(&mut self, x: usize, y: usize, text: String) -> (usize, usize) {
        let mut l = self.line_graphemes(y);
        let start = l.drain(..x).collect::<Vec<&str>>().concat();
        let end = l.concat();
        self.contents.remove(y);
        self.insert_line(y, start);
        let mut x = x;
        let mut y = y;
        for c in text.chars() {
            if c == '\n' {
                y += 1;
                x = 0;
                self.insert_line(y, String::new());
            } else {
                self.contents[y].push(c);
                x += 1;
            }
        }
        self.contents[y].push_str(&end);
        (x, y)
    }

    fn undo_action(&mut self, a: Action) -> Action {
        Action {
            inserted_text: match a.deleted_text {
                Some(text) => {
                    self.do_insert(a.x1, a.y1, text.clone());
                    Some(text)
                }
                None => None,
            },
            deleted_text: match a.inserted_text {
                Some(_text) => Some(self.do_delete(a.x1, a.y1, a.x2, a.y2)),
                None => None,
            },
            x1: a.x1,
            y1: a.y1,
            x2: a.x2,
            y2: a.y2,
        }
    }

    pub fn undo(&mut self) {
        if let Some(a) = self.undo_stack.pop() {
            let a = self.undo_action(a);
            self.redo_stack.push(a);
        }
    }

    pub fn redo(&mut self) {
        if let Some(a) = self.redo_stack.pop() {
            let a = self.undo_action(a);
            self.undo_stack.push(a);
        }
    }

    pub fn next_char(&self, x: usize, y: usize) -> (usize, usize) {
        if x < self.line_len(y) {
            return (x + 1, y);
        } else if y < self.len() - 1 {
            return (0, y + 1);
        }
        (x, y)
    }

    pub fn prev_char(&self, x: usize, y: usize) -> (usize, usize) {
        if x > 0 {
            return (x - 1, y);
        } else if y > 0 {
            return (self.line_len(y - 1), y - 1);
        }
        (x, y)
    }

    pub fn next_word(&self, x: usize, y: usize) -> (usize, usize) {
        let mut bounds = self.contents[y]
            .split_word_bound_indices()
            .map(|(i, _word)| i)
            .collect::<Vec<usize>>();
        bounds.push(self.line_len(y));

        for i in bounds {
            if i > x {
                return (i, y);
            }
        }
        if y < self.len() - 1 {
            return (0, y + 1);
        }
        (x, y)
    }

    pub fn prev_word(&self, x: usize, y: usize) -> (usize, usize) {
        for (i, _words) in self.contents[y].split_word_bound_indices().rev() {
            if i < x {
                return (i, y);
            }
        }
        if y > 0 {
            return (self.line_len(y - 1), y - 1);
        }
        (x, y)
    }
}
