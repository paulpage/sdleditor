use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use sdl2::ttf::Font;
use std::cmp::{max, min};
use std::collections::HashMap;
use std::rc::Rc;

extern crate unicode_segmentation;
use unicode_segmentation::UnicodeSegmentation;

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
    pub scroll_idx: usize,
    pub scroll_offset: i32,
    pub line_height: i32,
    char_width: i32,
    pub font: Font<'a, 'static>,
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
            scroll_idx: 0,
            scroll_offset: 0,
            line_height: font.height(),
            char_width: font.size_of_char('o').unwrap().0 as i32,
            font,
            font_cache: HashMap::new(),
        }
    }

    pub fn draw(
        &mut self,
        mut canvas: &mut WindowCanvas,
        buffer: &Buffer,
        padding: i32,
        is_active: bool,
    ) {
        // Background
        let color_bg = Color::RGB(40, 40, 40);
        let color_fg = Color::RGB(253, 244, 193);
        let color_selection1 = Color::RGB(168, 153, 132);
        let (color_bar_bg, color_bar_fg) = if is_active {
            (Color::RGB(80, 73, 69), Color::RGB(253, 244, 193))
        } else {
            (Color::RGB(60, 56, 54), Color::RGB(189, 174, 147))
        };
        canvas.set_draw_color(color_bar_bg);
        let rect = Rect::new(
            self.x - padding,
            self.y - padding,
            self.w + padding as u32 * 2,
            self.h + padding as u32 * 2,
        );
        canvas.fill_rect(rect).unwrap();
        canvas.set_draw_color(color_bg);
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

        // We only want to render the lines that are actually on the screen.
        let (first_line, last_line) = self.get_lines_on_screen(&buffer);

        let bar_height: i32 = self.line_height as i32 + padding * 2;

        // TODO: text off bottom?
        // TODO: cursor
        // TODO: selection
        // TODO: factor wrapped lines into starting line
        let chars_per_line = max(1, (self.w - padding as u32 * 4) / self.char_width as u32);
        let mut y = 0;
        let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = buffer.get_selection();
        for (i, line) in buffer.contents[first_line..last_line].iter().enumerate() {
            let unicode_line = line.as_str().graphemes(true).collect::<Vec<&str>>();
            let midpoint = min(buffer.cursor_x, unicode_line.len());
            let mut x = 0;
            for c in unicode_line {

                // Draw selection
                // TODO check if it's actually the selection
                if i >= sel_start_y && i <= sel_end_y {
                    let rect = Rect::new(
                        (x * self.char_width as u32 + padding as u32 * 2) as i32,
                        y * self.line_height + bar_height + padding * 2,
                        self.char_width as u32,
                        self.line_height as u32,
                    );
                    self.fill_rect(&mut canvas, color_selection1, rect);
                }

                // Draw character
                if !c.trim().is_empty() {
                    self.draw_char(
                        canvas,
                        color_fg,
                        (x * self.char_width as u32 + padding as u32 * 2) as i32,
                        y * self.line_height + padding * 2 + bar_height,
                        c,
                    );
                }

                // Draw cursor
                if is_active && i == buffer.cursor_y {
                    let rect = Rect::new(
                        padding * 2 + midpoint as i32 * self.char_width as i32,
                        y * self.line_height + bar_height + padding * 2,
                        2,
                        self.line_height as u32,
                    );
                    self.fill_rect(&mut canvas, color_fg, rect);
                }
                
                x += 1;
                if x == chars_per_line {
                    x = 0;
                    y += 1;
                }


            }
            y += 1;
        }

        // Draw the contents of the file and the cursor.
        // let mut y = 0;
        // for (i, entry) in buffer.contents[first_line..last_line]
        //     .iter()
        //     .enumerate()
        //     .map(|(i, entry)| (i + first_line, entry))
        // {
        //     let uentry = entry.as_str().graphemes(true).collect::<Vec<&str>>();
        //     let midpoint = min(buffer.cursor_x, uentry.len());
        //     let line_y =
        //         bar_height + padding * 2 + y as i32 * self.line_height as i32 - self.scroll_offset;

        //     // Draw the selection
        //     let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = buffer.get_selection();
        //     if i >= sel_start_y && i <= sel_end_y {
        //         let mut x1: u32 = 0;
        //         let mut x2: u32 = self.text_length(&buffer.contents[i]);
        //         if !buffer.contents[i].is_empty() {
        //             if i == sel_start_y {
        //                 x1 = self.text_length(
        //                     &buffer.line_graphemes(i)[..sel_start_x].concat().to_string(),
        //                 );
        //             }
        //             if i == sel_end_y {
        //                 x2 = self.text_length(
        //                     &buffer.line_graphemes(i)[..sel_end_x].concat().to_string(),
        //                 );
        //             }
        //         }
        //         if x2 > x1 {
        //             let rect = Rect::new(
        //                 padding * 2 + x1 as i32,
        //                 line_y,
        //                 (x2 - x1) as u32,
        //                 self.line_height as u32,
        //             );
        //             self.fill_rect(&mut canvas, color_selection1, rect);
        //         }
        //     }

        //     // Draw the text
        //     let mut chars_rendered = 0;
        //     let mut midpoint_width: i32 = 0;
        //     while chars_rendered < midpoint {
        //         let (c, w) = self.draw_text(
        //             &mut canvas,
        //             color_fg,
        //             padding * 2,
        //             padding * 4,
        //             line_y + y as i32,
        //             &uentry[chars_rendered..midpoint],
        //         );
        //         chars_rendered += c;
        //         midpoint_width += w;
        //         if chars_rendered < midpoint {
        //             y += 1;
        //             midpoint_width = 0;
        //         }
        //         // if extra_lines > 0 {
        //         // }
        //         // extra_lines += 1;
        //     }
        //     // y = max(orig_y as i32, y as i32 - 1) as usize;
        //     // chars_rendered = 0;
        //     // while chars_rendered < uentry[midpoint..].len() {
        //     while chars_rendered < uentry.len() {
        //         let (c, w) = self.draw_text(
        //             &mut canvas,
        //             color_fg,
        //             padding * 2 + midpoint_width,
        //             padding * 4,
        //             // padding * 2,
        //             line_y + y as i32,
        //             &uentry[chars_rendered..],
        //         );
        //         chars_rendered += c;
        //         // midpoint_width += w;
        //         y += 1;
        //     }

        // // Draw the cursor
        // if is_active && i == buffer.cursor_y {
        //     let rect = Rect::new(
        //         padding * 2 + midpoint_width as i32,
        //         line_y,
        //         2,
        //         self.line_height as u32,
        //     );
        //     self.fill_rect(&mut canvas, color_fg, rect);
        // }
        // }

        // Draw the bar
        let rect = Rect::new(0, 0, self.w, bar_height as u32);
        self.fill_rect(&mut canvas, color_bar_bg, rect);
        let dirty_text = if buffer.is_dirty { "*" } else { "" };
        let bar_text = vec![dirty_text, " ", &buffer.name];
        self.draw_text(
            &mut canvas,
            color_bar_fg,
            padding,
            padding,
            padding,
            &bar_text[..],
        );
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
        self.scroll_idx = min(
            buffer.len(),
            max(0, self.scroll_idx as i32 + lines) as usize,
        );
    }

    pub fn draw_text(
        &mut self,
        canvas: &mut WindowCanvas,
        color: Color,
        x: i32,
        padding: i32,
        y: i32,
        text: &[&str],
    ) -> (usize, i32) {
        let mut length: i32 = 0;
        if y > 0 && x > 0 {
            for (i, c) in text.iter().filter(|x| !x.is_empty()).enumerate() {
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
                let w = min(self.w as i32 - (x + length) as i32, tex.w as i32) as u32;
                let h = min(self.h as i32 - y as i32, tex.h as i32) as u32;
                length += w as i32;
                if length > self.w as i32 - padding {
                    return (i, self.w as i32);
                }
                let source = Rect::new(0, 0, w, h);
                let target = Rect::new(self.x + x + length as i32, self.y + y, w, h);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();
            }
        }
        (text.len(), length)
    }

    pub fn get_lines_on_screen(&self, buffer: &Buffer) -> (usize, usize) {
        let scroll_delta = self.scroll_idx as i32 * self.line_height as i32 - self.scroll_offset;
        let num_lines = ((f64::from(self.h) + f64::from(scroll_delta.abs()))
            / f64::from(self.line_height))
        .ceil() as usize;
        let first = max(0, self.scroll_idx as i32 - num_lines as i32) as usize;
        let last = min(buffer.len(), self.scroll_idx + num_lines);
        (first, last)
    }

    pub fn select_all(&mut self, buffer: &mut Buffer) {
        buffer.select_all();
        let (first, _last) = self.get_lines_on_screen(buffer);
        self.scroll_idx = first;
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
        let padding = 10;
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
        let mut x_idx: usize = 0;
        let mut last_length = length;
        while length < x && x_idx < max_x_idx {
            last_length = length;
            let (char_x, _) = self
                .font
                .size_of(buffer.line_graphemes(y_idx)[x_idx])
                .unwrap();
            length += char_x as i32;
            x_idx += 1;
        }
        if (last_length as i32 - x as i32).abs() > (length as i32 - x as i32).abs() {
            x_idx += 1;
        }
        x_idx = max(x_idx as i32 - 1, 0) as usize;

        buffer.cursor_x = x_idx;
        buffer.cursor_y = y_idx;
        buffer.set_selection(extend);
    }
}
