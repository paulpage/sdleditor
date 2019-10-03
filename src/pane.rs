use std::collections::HashMap;
use std::rc::Rc;
use std::cmp::{min, max};
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use sdl2::rect::Rect;
use sdl2::pixels::Color;

// mod buffer;
use crate::buffer::Buffer;

pub enum PaneType {
    Buffer,
    FileManager,
}

#[derive(Hash)]
struct FontCacheKey {
    c: char,
    color: Color,
}

impl PartialEq for FontCacheKey {
    fn eq(&self, other: &Self) -> bool {
        self.c == other.c && self.color == other.color
    }
}

impl Eq for FontCacheKey {}

pub struct Pane<'a> {
    pub pane_type: PaneType,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub buffer_id: Option<usize>,
    pub cursor_x: usize,
    pub max_cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_idx: usize,
    pub scroll_offset: i32,
    pub line_height: i32,
    pub font: Font<'a, 'static>,
    pub sel_x: usize,
    pub sel_y: usize,
    font_cache: HashMap<FontCacheKey, Rc<Texture>>,
}

impl<'a> Pane<'a> {

    pub fn new(x: i32, y: i32, w: u32, h: u32, font: Font<'a, 'static>, pane_type: PaneType, buffer_id: Option<usize>) -> Self {
        Pane {
            pane_type: pane_type,
            x: x,
            y: y,
            w: w,
            h: h,
            buffer_id: buffer_id,
            cursor_x: 0,
            cursor_y: 0,
            max_cursor_x: 0,
            sel_x: 0,
            sel_y: 0,
            scroll_idx: 0,
            scroll_offset: 0,
            line_height: font.height(),
            font: font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(&self, canvas: &mut WindowCanvas) {
        let rect = Rect::new(self.x, self.y, self.w, self.h);
        canvas.fill_rect(rect).unwrap();
    }

    pub fn fill_rect(&mut self, canvas: &mut WindowCanvas, color: Color, rect: Rect) {
        canvas.set_draw_color(color);
        let x = min(self.x + self.w as i32, max(self.x, self.x + rect.x));
        let y = min(self.y + self.h as i32, max(self.y, self.y + rect.y));
        let w = max(0, min(self.w as i32 - rect.x, rect.w + min(0, rect.x))) as u32;
        let h = max(0, min(self.h as i32 - rect.y, rect.h + min(0, rect.y))) as u32;
        if w > 0 && h > 0 {
            canvas.fill_rect(Rect::new(x, y, w, h)).unwrap();
        }
    }

    pub fn draw_text(&mut self, canvas: &mut WindowCanvas, color: Color, x: i32, y: i32, text: &str) -> i32 {
        let mut length: i32 = 0;
        if y > 0 && x > 0 {
            for c in text.chars() {
                let key = FontCacheKey {c: c, color: color };
                let texture = self
                    .font_cache
                    .get(&key)
                    .cloned()
                    .unwrap_or_else(|| {
                        let surface = self
                            .font
                            .render(&c.to_string())
                            .blended(color)
                            .unwrap();
                        let texture = canvas
                            .texture_creator()
                            .create_texture_from_surface(&surface)
                            .unwrap();
                        let resource = Rc::new(texture);
                        self.font_cache.insert(key, resource.clone());
                        resource
                    });

                let TextureQuery {
                    width: mut w,
                    height: mut h,
                    ..
                } = texture.query();
                w = min(self.w as i32 - (x + length) as i32, w as i32) as u32;
                h = min(self.h as i32 - y as i32, h as i32) as u32;
                let source = Rect::new(0, 0, w as u32, h as u32);
                let target = Rect::new(self.x + x + length as i32, self.y + y, w, h);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();

                if length > self.w as i32 {
                    return self.w as i32;
                }
                length += w as i32;
            }
        }
        length
    }

    pub fn cursor_up(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        self.cursor_y = max(0, self.cursor_y as i32 - num as i32) as usize;
        self.cursor_x = max(
            min(self.cursor_x, buffer.contents[self.cursor_y].len()),
            min(self.max_cursor_x, buffer.contents[self.cursor_y].len()),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_down(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        self.cursor_y = min(buffer.contents.len() - 1, self.cursor_y + num);
        self.cursor_x = max(
            min(self.cursor_x, buffer.contents[self.cursor_y].len()),
            min(self.max_cursor_x, buffer.contents[self.cursor_y].len()),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_left(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        if self.cursor_x as i32 - num as i32 >= 0 {
            self.cursor_x = (self.cursor_x as i32 - num as i32) as usize;
        } else {
            if self.cursor_y > 0 {
                let remainder = ((self.cursor_x as i32 - num as i32).abs() - 1) as usize;
                self.cursor_up(1, buffer, extend_selection);
                self.cursor_x = buffer.contents[self.cursor_y].len();
                self.cursor_left(remainder, buffer, extend_selection);
            }
        }
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn cursor_right(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        if self.cursor_x + num <= buffer.contents[self.cursor_y].len() {
            self.cursor_x += num;
        } else {
            if self.cursor_y < buffer.contents.len() - 1 {
                let remainder = (((self.cursor_x + num) as i32 - buffer.contents[self.cursor_y].len() as i32).abs() - 1) as usize;
                self.cursor_down(1, buffer, extend_selection);
                self.cursor_x = 0;
                self.cursor_right(remainder, buffer, extend_selection);
            }
        }
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn scroll_up(&mut self, num: usize) {
        self.scroll_idx = max(0, self.scroll_idx as i32 - num as i32) as usize;
    }

    pub fn scroll_down(&mut self, num: usize, buffer: &Buffer) {
        self.scroll_idx = min(buffer.contents.len(), self.scroll_idx + num);
    }

    pub fn break_line(&mut self, buffer: &mut Buffer) {
        let first_half = buffer.contents[self.cursor_y][0..self.cursor_x].to_string();
        let last_half = buffer.contents[self.cursor_y][self.cursor_x..].to_string();
        buffer.contents[self.cursor_y] = first_half;
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.max_cursor_x = self.cursor_x;
        buffer.contents.insert(self.cursor_y, last_half);
    }

    pub fn remove_char(&mut self, mut buffer: &mut Buffer) {
        if self.cursor_x > 0 {
            if self.cursor_x <= buffer.contents[self.cursor_y].len() {
                buffer.contents[self.cursor_y].remove(self.cursor_x - 1);
            }
            self.cursor_x -= 1;
            self.max_cursor_x = self.cursor_x;
            buffer.is_dirty = true;
        } else {
            if self.cursor_y > 0 {
                let this_line = buffer.contents.remove(self.cursor_y);
                self.cursor_y -= 1;
                self.cursor_x = buffer.contents[self.cursor_y].len();
                buffer.contents[self.cursor_y].push_str(&this_line);
            }
        }
    }

    pub fn text_length(&self, text: &str) -> u32 {
        let mut length = 0;
        for c in text.chars() {
            let (x, _) = self.font.size_of_char(c).unwrap();
            length += x;
        }
        length
    }

    pub fn set_selection(&mut self, extend_selection: bool) {
        if !extend_selection {
            self.sel_x = self.cursor_x;
            self.sel_y = self.cursor_y;
        }
    }

    // A selection is defined by the cursor position as one corner
    // and the selection position at the other. This function
    // returns those corners in order: (x1, y1, x2, y2)
    // where x1 and y1 are earlier in the buffer than x2 and y2
    pub fn get_selection(&self) -> (usize, usize, usize, usize) {
        if self.sel_y > self.cursor_y || self.sel_y == self.cursor_y
            && self.sel_x > self.cursor_x {
            return (self.cursor_x, self.cursor_y, self.sel_x, self.sel_y);
        }
        return (self.sel_x, self.sel_y, self.cursor_x, self.cursor_y);
    }

    pub fn remove_selection(&mut self, buffer: &mut Buffer) {
        let (x1, y1, x2, y2) = self.get_selection();
        if x1 == x2 && y1 == y2 {
            self.remove_char(buffer);
        } else {
            buffer.delete(x1, y1, x2, y2);
            self.cursor_x = x1;
            self.cursor_y = y1;
        }
        self.set_selection(false);
    }
    
    // Get the buffer indices based on a screen position. This can be used to set
    // the cursor position with the mouse.
    pub fn get_position_from_screen(&self, x: i32, y: i32, buffer: &Buffer) -> (usize, usize) {
        let padding = 5;
        let bar_height: u32 = (self.line_height + padding * 2) as u32;
        let mut y_idx = max(((f64::from(y)
                    - f64::from(self.y)
                    - f64::from(padding)
                    - f64::from(bar_height))
                / f64::from(self.line_height))
            .floor() as i32, 0) as usize
            + self.scroll_idx;
        y_idx = min(y_idx, buffer.contents.len() - 1);
        let max_x_idx = buffer.contents[y_idx].len();

        let mut length = self.x + padding;
        let mut x_idx = 0;
        let mut last_length = length;
        while length < x && (x_idx as usize) < max_x_idx {
            last_length = length;
            let (char_x, _) =  self.font
                .size_of(&buffer.contents[y_idx].chars().nth(x_idx).unwrap().to_string())
                .unwrap();
            length += char_x as i32;
            x_idx += 1;
        }
        if (last_length as i32 - x as i32).abs() > (length as i32 - x as i32).abs() {
            x_idx += 1;
        }
        x_idx = max(x_idx as i32 - 1, 0) as usize;

        (x_idx, y_idx)
        // self.cursor_x = max(x_idx as i32 - 1, 0) as usize;
        // self.max_cursor_x = self.cursor_x;
        // self.set_selection(false);
    }
}

