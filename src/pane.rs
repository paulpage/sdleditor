use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::rc::Rc;

use clipboard::{ClipboardContext, ClipboardProvider};

extern crate unicode_segmentation;

use crate::buffer::Buffer;

pub enum PaneType {
    Buffer,
    FileManager,
}

#[derive(Hash, PartialEq)]
struct FontCacheKey {
    c: String,
    color: Color,
}

struct FontCacheEntry {
    texture: Texture,
    w: u32,
    h: u32,
}

impl Eq for FontCacheKey {}

pub struct Pane<'a> {
    pub pane_type: PaneType,
    pub x: i32,
    pub y: i32,
    pub w: u32,
    pub h: u32,
    pub buffer_id: usize,
    pub cursor_x: usize,
    pub max_cursor_x: usize,
    pub cursor_y: usize,
    pub scroll_idx: usize,
    pub scroll_offset: i32,
    pub line_height: i32,
    pub font: Font<'a, 'static>,
    pub sel_x: usize,
    pub sel_y: usize,
    font_cache: HashMap<FontCacheKey, Rc<FontCacheEntry>>,
}

impl<'a> Pane<'a> {
    pub fn new(font: Font<'a, 'static>, pane_type: PaneType, buffer_id: usize) -> Self {
        Pane {
            pane_type,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            buffer_id,
            cursor_x: 0,
            cursor_y: 0,
            max_cursor_x: 0,
            sel_x: 0,
            sel_y: 0,
            scroll_idx: 0,
            scroll_offset: 0,
            line_height: font.height(),
            font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(&mut self, canvas: &mut WindowCanvas, padding: i32, is_active: bool) {
        // Background
        let border_color = if is_active {
            Color::RGBA(152, 151, 26, 255)
        } else {
            Color::RGBA(0, 0, 0, 255)
        };
        canvas.set_draw_color(border_color);
        let rect = Rect::new(
            self.x - padding,
            self.y - padding,
            self.w + padding as u32 * 2,
            self.h + padding as u32 * 2,
        );
        canvas.fill_rect(rect).unwrap();
        canvas.set_draw_color(Color::RGBA(20, 20, 20, 255));
        let rect = Rect::new(self.x, self.y, self.w, self.h);
        canvas.fill_rect(rect).unwrap();

        // Calculate smooth scrolling
        let target_scroll_offset = self.scroll_idx as i32 * self.line_height as i32;
        let scroll_delta = target_scroll_offset - self.scroll_offset;
        if self.scroll_offset < target_scroll_offset {
            self.scroll_offset += (f64::from(scroll_delta) / 3.0).ceil() as i32;
        } else if self.scroll_offset > target_scroll_offset {
            self.scroll_offset += (f64::from(scroll_delta) / 3.0).floor() as i32;
        }
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

    pub fn draw_text(
        &mut self,
        canvas: &mut WindowCanvas,
        color: Color,
        x: i32,
        y: i32,
        text: &[&str],
    ) -> i32 {
        let mut length: i32 = 0;
        if y > 0 && x > 0 {
            for c in text.iter().filter(|x| !x.is_empty()) {
                let key = FontCacheKey {
                    c: c.to_string(),
                    color,
                };
                let tex = self.font_cache.get(&key).cloned().unwrap_or_else(|| {
                    let surface = self.font.render(&c.to_string()).blended(color).unwrap();
                    let texture = canvas
                        .texture_creator()
                        .create_texture_from_surface(&surface)
                        .unwrap();
                    let TextureQuery { width, height, .. } = texture.query();
                    let resource = Rc::new(FontCacheEntry {texture, w: width, h: height });
                    self.font_cache.insert(key, resource.clone());
                    resource
                });

                let texture = &tex.texture;
                let w = min(self.w as i32 - (x + length) as i32, tex.w as i32) as u32;
                let h = min(self.h as i32 - y as i32, tex.h as i32) as u32;
                let source = Rect::new(0, 0, w, h);
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

    pub fn clipboard_paste(&mut self, buffer: &mut Buffer) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        if let Ok(s) = ctx.get_contents() {
            buffer.insert_text(self.cursor_x, self.cursor_y, s);
        }
    }

    pub fn clipboard_copy(&mut self, buffer: &mut Buffer) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let (x1, y1, x2, y2) = self.get_selection();
        let s = buffer.do_delete(x1, y1, x2, y2);
        buffer.do_insert(x1, y2, s.clone());
        ctx.set_contents(s).unwrap();
    }

    pub fn clipboard_cut(&mut self, buffer: &mut Buffer) {
        let mut ctx: ClipboardContext = ClipboardProvider::new().unwrap();
        let (x1, y1, x2, y2) = self.get_selection();
        let s = buffer.do_delete(x1, y1, x2, y2);
        ctx.set_contents(s).unwrap();
    }

    pub fn get_lines_on_screen(&self, buffer: &Buffer) -> (usize, usize) {
        let scroll_delta = self.scroll_idx as i32 * self.line_height as i32 - self.scroll_offset;
        let num_lines = ((f64::from(self.h) + f64::from(scroll_delta.abs()))
            / f64::from(self.line_height)).ceil() as usize;
        let first = max(0, self.scroll_idx as i32 - num_lines as i32) as usize;
        let last = min(buffer.len(), self.scroll_idx + num_lines);
        (first, last)
    }

    pub fn select_all(&mut self, buffer: &Buffer) {
        self.sel_y = 0;
        self.sel_x = 0;
        self.cursor_y = max(0, buffer.len() as i32 - 1) as usize;
        self.cursor_x = buffer.line_len(self.cursor_y);
        let (first, _last) = self.get_lines_on_screen(buffer);
        self.scroll_idx = first;
        self.set_selection(true);
    }

    pub fn select_line(&mut self, line: usize, buffer: &Buffer) {
        self.cursor_y = min(line, max(0, buffer.len() as i32 - 1) as usize);
        self.sel_y = self.cursor_y;
        self.cursor_x = buffer.line_len(self.sel_y);
        self.sel_x = 0;
    }

    pub fn cursor_up(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        self.cursor_y = max(0, self.cursor_y as i32 - num as i32) as usize;
        self.cursor_x = max(
            min(self.cursor_x, buffer.line_len(self.cursor_y)),
            min(self.max_cursor_x, buffer.line_len(self.cursor_y)),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_down(&mut self, num: usize, buffer: &Buffer, extend_selection: bool) {
        self.cursor_y = min(buffer.len() - 1, self.cursor_y + num);
        self.cursor_x = max(
            min(self.cursor_x, buffer.line_len(self.cursor_y)),
            min(self.max_cursor_x, buffer.line_len(self.cursor_y)),
        );
        self.set_selection(extend_selection);
    }

    pub fn cursor_left(&mut self, buffer: &Buffer, extend_selection: bool) {
        let (x, y) = buffer.prev_char(self.cursor_x, self.cursor_y);
        self.cursor_x = x;
        self.cursor_y = y;
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn cursor_right(&mut self, buffer: &Buffer, extend_selection: bool) {
        let (x, y) = buffer.next_char(self.cursor_x, self.cursor_y);
        self.cursor_x = x;
        self.cursor_y = y;
        self.max_cursor_x = self.cursor_x;
        self.set_selection(extend_selection);
    }

    pub fn scroll_up(&mut self, num: usize) {
        self.scroll_idx = max(0, self.scroll_idx as i32 - num as i32) as usize;
    }

    pub fn scroll_down(&mut self, num: usize, buffer: &Buffer) {
        self.scroll_idx = min(buffer.len(), self.scroll_idx + num);
    }

    pub fn break_line(&mut self, buffer: &mut Buffer) {
        let mut g = buffer.line_graphemes(self.cursor_y);
        let first_half = g
            .drain(..self.cursor_x)
            .collect::<Vec<&str>>()
            .concat()
            .to_string();
        let last_half = g.concat().to_string();
        buffer.contents[self.cursor_y] = first_half;
        self.cursor_y += 1;
        self.cursor_x = 0;
        self.max_cursor_x = self.cursor_x;
        buffer.insert_line(self.cursor_y, last_half);
        self.set_selection(false);
    }

    pub fn remove_char(&mut self, buffer: &mut Buffer) {
        let (x1, y1) = buffer.prev_char(self.cursor_x, self.cursor_y);
        buffer.delete_text(x1, y1, self.cursor_x, self.cursor_y);
        self.cursor_x = x1;
        self.cursor_y = y1;
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
        if self.sel_y > self.cursor_y || self.sel_y == self.cursor_y && self.sel_x > self.cursor_x {
            return (self.cursor_x, self.cursor_y, self.sel_x, self.sel_y);
        }
        (self.sel_x, self.sel_y, self.cursor_x, self.cursor_y)
    }

    pub fn remove_selection(&mut self, buffer: &mut Buffer) {
        let (x1, y1, x2, y2) = self.get_selection();
        if x1 == x2 && y1 == y2 {
            self.remove_char(buffer);
        } else {
            buffer.delete_text(x1, y1, x2, y2);
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
        let mut y_idx = max(
            ((f64::from(y) - f64::from(self.y) - f64::from(padding) - f64::from(bar_height))
                / f64::from(self.line_height))
            .floor() as i32,
            0,
        ) as usize
            + self.scroll_idx;
        y_idx = min(y_idx, buffer.len() - 1);
        let max_x_idx = buffer.line_len(y_idx);

        let mut length = self.x + padding;
        let mut x_idx = 0;
        let mut last_length = length;
        while length < x && (x_idx as usize) < max_x_idx {
            last_length = length;
            let (char_x, _) = self
                .font
                .size_of(
                    &buffer.contents[y_idx]
                        .chars()
                        .nth(x_idx)
                        .unwrap()
                        .to_string(),
                )
                .unwrap();
            length += char_x as i32;
            x_idx += 1;
        }
        if (last_length as i32 - x as i32).abs() > (length as i32 - x as i32).abs() {
            x_idx += 1;
        }
        x_idx = max(x_idx as i32 - 1, 0) as usize;
        (x_idx, y_idx)
    }
}
