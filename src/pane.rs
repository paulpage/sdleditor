use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::rc::Rc;

use regex::Regex;

use unicode_segmentation::UnicodeSegmentation;

use crate::buffer::Buffer;

struct ColorScheme {
    fg: Color,
    bg: Color,
    ui_fg: Color,
    ui_bg: Color,
    ui_inactive_fg: Color,
    ui_inactive_bg: Color,
    selection: Color,
    comment: Color,
}

struct Syntax {
    line_comment: Regex,
    block_comment_start: Regex,
    block_comment_end: Regex,
    keywords: Vec<String>,
    string_start: Regex,
    string_end: Regex,
    has_nested_comments: bool,
}

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
    // pub scroll: i32,
    pub scroll_offset: i32,
    pub scroll_lag: i32,
    pub line_height: i32,
    char_width: i32,
    pub font: Font<'a, 'static>,
    font_cache: HashMap<FontCacheKey, Rc<FontCacheEntry>>,
    syntax: Syntax,
    colors: ColorScheme,
}

impl<'a> Pane<'a> {
    pub fn new(font: Font<'a, 'static>, pane_type: PaneType, buffer_id: usize) -> Self {
        // TODO Very incomplete
        let syntax = Syntax {
            line_comment: Regex::new(r"//").unwrap(),
            block_comment_start: Regex::new(r"/\*").unwrap(),
            block_comment_end: Regex::new(r"\*/").unwrap(),
            string_start: Regex::new(r#"""#).unwrap(),
            string_end: Regex::new(r#"""#).unwrap(),
            keywords: vec!["fn".into(), "let".into()],
            has_nested_comments: true,
        };
        let colors = ColorScheme {
            fg: Color::RGB(253, 244, 193),
            bg: Color::RGB(40, 40, 40),
            ui_fg: Color::RGB(253, 244, 193),
            ui_bg: Color::RGB(80, 73, 69),
            ui_inactive_fg: Color::RGB(189, 174, 147),
            ui_inactive_bg: Color::RGB(60, 56, 54),
            selection: Color::RGB(168, 153, 132),
            comment: Color::RGB(168, 153, 132), // TODO same as selection
        };

        Pane {
            pane_type,
            x: 0,
            y: 0,
            w: 0,
            h: 0,
            buffer_id,
            // scroll: 0,
            scroll_lag: 0,
            scroll_offset: 0,
            line_height: font.height(),
            char_width: font.size_of_char('o').unwrap().0 as i32,
            font,
            font_cache: HashMap::new(),
            syntax,
            colors,
        }
    }

    pub fn draw(&mut self, mut canvas: &mut WindowCanvas, buffer: &Buffer, is_active: bool) {
        let padding = 5;
        canvas.set_draw_color(self.colors.ui_bg);
        let rect = Rect::new(
            self.x - padding,
            self.y - padding,
            self.w + padding as u32 * 2,
            self.h + padding as u32 * 2,
        );
        canvas.fill_rect(rect).unwrap();
        canvas.set_draw_color(self.colors.bg);
        let rect = Rect::new(self.x, self.y, self.w, self.h);
        canvas.fill_rect(rect).unwrap();

        if self.scroll_lag != 0 {
            let scroll_pixels = min(
                max(self.line_height / 2, self.scroll_lag.abs() / 3),
                self.scroll_lag.abs(),
            );
            let direction = self.scroll_lag / self.scroll_lag.abs();
            self.scroll_offset += scroll_pixels * direction;
            self.scroll_lag -= scroll_pixels * direction;
        }

        let bar_height: i32 = self.line_height as i32 + padding * 2;

        let mut color = self.colors.fg;
        let mut comment_level = 0;

        let chars_per_line = max(1, (self.w - padding as u32 * 4) / self.char_width as u32);
        let mut y = 0;
        let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = buffer.get_selection();
        for (i, line) in buffer.contents.iter().enumerate() {

            let has_line_comment = self.syntax.line_comment.is_match(line);
            let has_block_comment_start = self.syntax.block_comment_start.is_match(line);
            let has_block_comment_end = self.syntax.block_comment_end.is_match(line);
            let has_line_comment = self.syntax.line_comment.is_match(line);
            let block_comment_start = if has_block_comment_start {
                self.syntax.block_comment_start.find_iter(line).collect::<Vec<_>>()
            } else {
                vec![]
            };
            let block_comment_end = if has_block_comment_end {
                self.syntax.block_comment_end.find_iter(line).collect::<Vec<_>>()
            } else {
                vec![]
            };
            let mut is_line_comment = false;

            if y < self.scroll_offset + self.h as i32 {
                let mut unicode_line = line.as_str().graphemes(true).collect::<Vec<&str>>();
                // Needed to draw cursor even if we're on a blank line
                unicode_line.push(" ");
                // if unicode_line.len() == 0 {
                //     unicode_line = vec![" "];
                // }
                let mut x = 0;
                for (j, c) in unicode_line.iter().enumerate() {

                    for m in &block_comment_start {
                        if j == m.start() {
                            comment_level += 1;
                        }
                    }
                    for m in &block_comment_end {
                        if j == m.end() {
                            comment_level -= 1;
                        }
                    }
                    if comment_level > 0 {
                        color = self.colors.comment;
                    } else {
                        color = self.colors.fg;
                        if let Some(m) = self.syntax.line_comment.find(line) {
                            let mut block_overlaps = false;
                            for m in &block_comment_end {
                                if j == m.end() {
                                    block_overlaps = true;
                                }
                            }
                            if j == m.start() {
                                if !has_block_comment_end || !block_overlaps {
                                    is_line_comment = true;
                                }
                            }
                        }
                    }
                    if is_line_comment {
                        color = self.colors.comment;
                    }

                    // Draw selection
                    if i >= sel_start_y && i <= sel_end_y {
                        if (j >= sel_start_x || i > sel_start_y) && (j < sel_end_x || i < sel_end_y)
                        {
                            let rect = Rect::new(
                                (x * self.char_width as u32 + padding as u32 * 2) as i32,
                                y - self.scroll_offset + bar_height + padding * 2,
                                self.char_width as u32,
                                self.line_height as u32,
                            );
                            self.fill_rect(&mut canvas, self.colors.selection, rect);
                        }
                    }

                    // Draw character
                    if y >= self.scroll_offset {
                        if !c.trim().is_empty() {
                            self.draw_char(
                                canvas,
                                color,
                                (x * self.char_width as u32 + padding as u32 * 2) as i32,
                                y - self.scroll_offset + padding * 2 + bar_height,
                                c,
                            );
                        }
                    }

                    // Draw cursor
                    if is_active && i == buffer.cursor_y && j == buffer.cursor_x {
                        let rect = Rect::new(
                            (x * self.char_width as u32 + padding as u32 * 2) as i32,
                            y - self.scroll_offset + padding * 2 + bar_height,
                            2,
                            self.line_height as u32,
                        );
                        self.fill_rect(&mut canvas, self.colors.fg, rect);
                    }

                    x += 1;
                    if x == chars_per_line {
                        x = 0;
                        y += self.line_height;
                    }
                }
                y += self.line_height;
            }
        }

        // Draw the bar
        let rect = Rect::new(0, 0, self.w, bar_height as u32);
        self.fill_rect(&mut canvas, self.colors.ui_bg, rect);
        let dirty_text = if buffer.is_dirty { "*" } else { "" };
        let bar_text = vec![dirty_text, " ", &buffer.name];
        for (i, c) in bar_text.iter().filter(|x| !x.is_empty()).enumerate() {
            self.draw_char(canvas, self.colors.ui_fg, i as i32 * self.char_width + padding, padding, c);
        }
    }

    pub fn handle_keystroke(&mut self, buffer: &mut Buffer, kstr: &str) -> bool {
        match kstr {
            "Up" => buffer.cursor_up(1, false),
            "Down" => buffer.cursor_down(1, false),
            "Left" => buffer.cursor_left(false),
            "Right" => buffer.cursor_right(false),
            "PageUp" => self.scroll(buffer, -40),
            "PageDown" => self.scroll(buffer, 40),
            "Return" => buffer.break_line(),
            "S-Return" => buffer.break_line(),
            "Backspace" => buffer.remove_selection(),
            "S-Backspace" => buffer.remove_selection(),
            "Tab" => buffer.action_insert_text("    ".to_string()),
            "S-Up" => buffer.cursor_up(1, true),
            "S-Down" => buffer.cursor_down(1, true),
            "S-Left" => buffer.cursor_left(true),
            "S-Right" => buffer.cursor_right(true),
            "C-A" => self.select_all(buffer),
            "C-C" => buffer.clipboard_copy(),
            "C-S" => buffer.save(),
            "C-V" => buffer.clipboard_paste(),
            "C-X" => buffer.clipboard_cut(),
            "C-Z" => buffer.undo(),
            "C-Up" => buffer.cursor_up(1, false),
            "C-Down" => buffer.cursor_down(1, false),
            "C-S-Up" => buffer.cursor_up(1, true),
            "C-S-Down" => buffer.cursor_down(1, true),
            "C-Right" => {
                let (x, y) = buffer.next_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.set_selection(false);
            }
            "C-Left" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.set_selection(false);
            }
            "C-S-Right" => {
                let (x, y) = buffer.next_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
            }
            "C-S-Left" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
            }
            "C-S-Z" => buffer.redo(),
            "C-S-\\" => {
                buffer.print();
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn draw_char(&mut self, canvas: &mut WindowCanvas, color: Color, x: i32, y: i32, c: &str) {
        canvas.set_draw_color(color);
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
            let resource = Rc::new(FontCacheEntry {
                texture,
                w: width,
                h: height,
            });
            self.font_cache.insert(key, resource.clone());
            resource
        });
        let texture = &tex.texture;
        let w = min(self.w as i32 - (x + self.char_width) as i32, tex.w as i32) as u32;
        let h = min(self.h as i32 - y as i32, tex.h as i32) as u32;
        let source = Rect::new(0, 0, w, h);
        let target = Rect::new(self.x + x, self.y + y, w, h);
        canvas.copy(&texture, Some(source), Some(target)).unwrap();
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

    pub fn scroll(&mut self, buffer: &Buffer, lines: i32) {
        // TODO We want to clamp this to the top and bottom of the buffer,
        // but it's not as easy as it used to be because of line wrapping
        self.scroll_lag += lines * self.line_height;
    }

    pub fn select_all(&mut self, buffer: &mut Buffer) {
        // TODO scroll to end of file?
        buffer.select_all();
    }

    pub fn text_length(&self, text: &str) -> u32 {
        let mut length = 0;
        for c in text.chars() {
            let (x, _) = self.font.size_of_char(c).unwrap();
            length += x;
        }
        length
    }

    // Set the selection/cursor positions based on screen coordinates.
    pub fn set_selection_from_screen(
        &mut self,
        mut buffer: &mut Buffer,
        x: i32,
        y: i32,
        extend: bool,
    ) {
        // TODO This is still broken when I scroll down
        let padding = 5;
        let chars_per_line = max(1, (self.w - padding as u32 * 4) / self.char_width as u32);
        let bar_height: u32 = (self.line_height + padding * 2) as u32;

        let x_cell = ((x - padding * 2 - (self.char_width / 2)) / self.char_width) as usize;
        let y_cell = ((y - padding - bar_height as i32 - (self.line_height / 2)) / self.line_height)
            as usize;

        let mut x_target = 0;
        let mut y_target = 0;
        let mut current_y = 0;
        'main: for (i, line) in buffer.contents.iter().enumerate() {
            let mut unicode_line = line.as_str().graphemes(true).collect::<Vec<&str>>();
            unicode_line.push(" ");
            let mut x = 0;
            for (j, _) in unicode_line.iter().enumerate() {
                let current_y_cell = (current_y - self.scroll_offset) / self.line_height;
                if current_y_cell == y_cell as i32 {
                    x_target = j;
                    y_target = i;
                    if x as usize == x_cell {
                        break 'main;
                    }
                }
                if current_y_cell > y_cell as i32 {
                    break 'main;
                }

                x += 1;
                if x == chars_per_line {
                    x = 0;
                    current_y += self.line_height;
                }
            }
            if (current_y - self.scroll_offset) / self.line_height as i32 >= y_cell as i32 {
                y_target = i;
                break 'main;
            }
            current_y += self.line_height;
        }
        buffer.cursor_x = x_target;
        buffer.cursor_y = y_target;
        buffer.set_selection(extend);
    }
}
