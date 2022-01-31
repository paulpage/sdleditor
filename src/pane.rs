use std::cmp::{max, min};

use regex::Regex;

use unicode_segmentation::UnicodeSegmentation;

use crate::buffer::Buffer;

use crate::canvas::{Canvas, Rect, Color};

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

pub struct Pane {
    pub pane_type: PaneType,
    pub rect: Rect,
    pub buffer_id: usize,
    // pub scroll: i32,
    pub scroll_offset: i32,
    pub scroll_lag: i32,
    pub line_height: i32,
    syntax: Syntax,
    colors: ColorScheme,
}

impl Pane {
    pub fn new(pane_type: PaneType, buffer_id: usize, line_height: i32) -> Self {
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
            rect: Rect::new(0, 0, 0, 0),
            buffer_id,
            // scroll: 0,
            scroll_lag: 0,
            scroll_offset: 0,
            line_height: line_height,
            syntax,
            colors,
        }
    }

    pub fn draw(&mut self, canvas: &mut Canvas, buffer: &Buffer, is_active: bool) {
        let padding = 5;

        // Fill background with border
        canvas.set_active_region(self.rect);
        canvas.fill_rect_with_border(Rect::new(0, 0, self.rect.w, self.rect.h), 5, self.colors.bg, self.colors.ui_bg);

        // Calculate scroll offset
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

        let chars_per_line = max(1, (self.rect.w - padding * 4) / canvas.char_width);
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

            if y < self.scroll_offset + self.rect.h as i32 {
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
                                x * canvas.char_width + padding * 2,
                                y - self.scroll_offset + bar_height + padding * 2,
                                canvas.char_width,
                                canvas.font_size,
                            );
                            canvas.fill_rect(rect, self.colors.selection);
                        }
                    }

                    // Draw character
                    if y >= self.scroll_offset {
                        if !c.trim().is_empty() {
                            canvas.draw_char(
                                color,
                                x * canvas.char_width + padding * 2,
                                y - self.scroll_offset + padding * 2 + bar_height,
                                c,
                            );
                        }
                    }

                    // Draw cursor
                    if is_active && i == buffer.cursor_y && j == buffer.cursor_x {
                        let rect = Rect::new(
                            x * canvas.char_width + padding * 2,
                            y - self.scroll_offset + padding * 2 + bar_height,
                            2,
                            canvas.font_size,
                        );
                        canvas.fill_rect(rect, self.colors.fg);
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
        let rect = Rect::new(0, 0, self.rect.w, bar_height);
        canvas.fill_rect(rect, self.colors.ui_bg);
        let dirty_text = if buffer.is_dirty { "*" } else { "" };
        let bar_text = vec![dirty_text, " ", &buffer.name];
        for (i, c) in bar_text.iter().filter(|x| !x.is_empty()).enumerate() {
            canvas.draw_char(self.colors.ui_fg, i as i32 * canvas.char_width + padding, padding, c);
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
            "Return" => buffer.break_line_with_auto_indent(),
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
            "C-Backspace" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.remove_selection();
            }
            "C-S-\\" => {
                buffer.print();
                return true;
            }
            _ => {}
        }
        false
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

    // Set the selection/cursor positions based on screen coordinates.
    pub fn set_selection_from_screen(
        &mut self,
        canvas: &Canvas,
        mut buffer: &mut Buffer,
        x: i32,
        y: i32,
        extend: bool,
    ) {
        // TODO This is still broken when I scroll down
        let padding = 5;
        let chars_per_line = max(1, (self.rect.w - padding * 4) / canvas.char_width);
        let bar_height = canvas.font_size + padding * 2;

        let x_cell = ((x - padding * 2 - (canvas.char_width / 2)) / canvas.char_width) as usize;
        let y_cell = ((y - padding - bar_height - (canvas.font_size / 2)) / canvas.font_size)
            as usize;

        let mut x_target = 0;
        let mut y_target = 0;
        let line_lengths = buffer.contents.iter().map(|line| line.as_str().graphemes(true).collect::<Vec<&str>>().len() + 1).collect::<Vec<usize>>();
        let line_count = line_lengths.len();
        if y_cell > 0 && y_cell < line_count {
            y_target = y_cell;
        } else if y_cell >= line_count {
            y_target = line_count - 1;
        }

        let char_count = line_lengths[y_target];
        if x_cell > 0 && x_cell < char_count {
            x_target = x_cell;
        } else if x_cell >= char_count {
            x_target = char_count - 1;
        }

        buffer.cursor_x = x_target;
        buffer.cursor_y = y_target;
        buffer.set_selection(extend);
    }
}
