use pgfx::{Engine, Color, Rect};
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

pub struct Pane {
    pub pane_type: PaneType,
    pub rect: Rect,
    pub buffer_id: usize,
    pub scroll_offset: f32,
    pub scroll_lag: f32,
    pub line_height: f32,
    syntax: Syntax,
    colors: ColorScheme,
    chars_per_line: i32,
    cursor_x: usize,
    cursor_y: usize,
    display_line_count: i32,
}

impl Pane {
    pub fn new(pane_type: PaneType, buffer_id: usize, line_height: f32) -> Self {
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
            fg: Color::new(253, 244, 193),
            bg: Color::new(40, 40, 40),
            ui_fg: Color::new(253, 244, 193),
            ui_bg: Color::new(80, 73, 69),
            ui_inactive_fg: Color::new(189, 174, 147),
            ui_inactive_bg: Color::new(60, 56, 54),
            selection: Color::new(168, 153, 132),
            comment: Color::new(168, 153, 132), // TODO same as selection
        };

        Pane {
            pane_type,
            rect: Rect::new(0.0, 0.0, 0.0, 0.0),
            buffer_id,
            scroll_lag: 0.0,
            scroll_offset: 0.0,
            line_height,
            syntax,
            colors,
            chars_per_line: 1,
            cursor_x: 0,
            cursor_y: 0,
            display_line_count: 0,
        }
    }

    pub fn draw(&mut self, app: &mut Engine, buffer: &Buffer, is_active: bool) {
        let padding = 5.0;

        // Fill background with border
        app.draw_rect(self.rect, self.colors.ui_bg);
        app.draw_rect(Rect::new(self.rect.x + 5.0, self.rect.y + 5.0, self.rect.width - 10.0, self.rect.height - 10.0), self.colors.bg);

        // Calculate scroll offset
        if self.scroll_lag != 0.0 {
            // let scroll_pixels = f32::min(
            //     f32::max(self.line_height / 2.0, self.scroll_lag.abs() / 3.0),
            //     self.scroll_lag.abs(),
            // );
            let scroll_pixels = self.scroll_lag.abs();
            let direction = self.scroll_lag / self.scroll_lag.abs();
            self.scroll_offset += scroll_pixels * direction;
            self.scroll_lag -= scroll_pixels * direction;
        }

        let bar_height = self.line_height + padding * 2.0;

        let mut color;
        let mut comment_level = 0;

        self.chars_per_line = f32::max(1.0, (self.rect.width - padding * 4.0) / app.char_width) as i32;
        let mut y = 0;
        let (sel_start_x, sel_start_y, sel_end_x, sel_end_y) = buffer.get_selection();
        for (i, line) in buffer.contents.iter().enumerate() {

            // let has_line_comment = self.syntax.line_comment.is_match(line);
            let has_block_comment_start = self.syntax.block_comment_start.is_match(line);
            let has_block_comment_end = self.syntax.block_comment_end.is_match(line);
            // let has_line_comment = self.syntax.line_comment.is_match(line);
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

            if y as f32 * self.line_height < self.scroll_offset + self.rect.height {
                let mut unicode_line = line.as_str().graphemes(true).collect::<Vec<&str>>();
                // Needed to draw cursor even if we're on a blank line
                unicode_line.push(" ");
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
                            if j == m.start() && (!has_block_comment_end || !block_overlaps) {
                                is_line_comment = true;
                            }
                        }
                    }
                    if is_line_comment {
                        color = self.colors.comment;
                    }

                    let screen_x = x as f32 * app.char_width + padding * 2.0;
                    let screen_y = y as f32 * self.line_height - self.scroll_offset + padding * 2.0 + bar_height;

                    // Set the selection for some reason
                    if screen_y + self.rect.y + app.font_size > app.mouse.y && screen_y + self.rect.y <= app.mouse.y {
                        self.cursor_y = i;
                        if screen_x + self.rect.x + app.char_width > app.mouse.x && screen_x + self.rect.x <= app.mouse.x {
                            self.cursor_x = j;
                        }
                    }

                    // Draw selection
                    if i >= sel_start_y && i <= sel_end_y && ((j >= sel_start_x || i > sel_start_y) && (j < sel_end_x || i < sel_end_y)) {
                        let rect = Rect::new(
                            self.rect.x + screen_x,
                            self.rect.y + screen_y,
                            app.char_width,
                            app.font_size,
                        );
                        app.draw_rect(rect, self.colors.selection);
                    }

                    // Draw character
                    if y as f32 * self.line_height >= self.scroll_offset - self.line_height && !c.trim().is_empty() {
                        app.draw_text(
                            c,
                            self.rect.x + screen_x,
                            self.rect.y + screen_y,
                            app.font_size,
                            color,
                        );
                    }

                    // Draw cursor
                    if is_active && i == buffer.cursor_y && j == buffer.cursor_x {
                        let rect = Rect::new(
                            self.rect.x + screen_x,
                            self.rect.y + screen_y,
                            2.0,
                            app.font_size,
                        );
                        app.draw_rect(rect, self.colors.fg);
                    }

                    x += 1;
                    if x == self.chars_per_line {
                        x = 0;
                        y += 1;
                    }
                }
            }
            // TODO This will make display_line_count almost accorate but not quite
            // because it won't account for wrapped lines. But right now we're only using
            // display_line_count to calculate scrolloff, so we don't really care about
            // wrapped lines that are off the end of the screen.
            y += 1;
        }
        self.display_line_count = y;

        // Draw the bar
        let rect = Rect::new(self.rect.x, self.rect.y, self.rect.width, bar_height);
        app.draw_rect(rect, self.colors.ui_bg);
        let dirty_text = if buffer.is_dirty { "*" } else { "" };
        let bar_text = vec![dirty_text, " ", &buffer.name];
        for (i, c) in bar_text.iter().filter(|x| !x.is_empty()).enumerate() {
            app.draw_text(c, self.rect.x + i as f32 * app.char_width + padding, self.rect.y + padding, app.font_size, self.colors.ui_fg);
        }
    }

    pub fn handle_keystroke(&mut self, buffer: &mut Buffer, kstr: &str) -> bool {
        match kstr {
            "up" => buffer.cursor_up(1, false),
            "down" => buffer.cursor_down(1, false),
            "left" => buffer.cursor_left(false),
            "right" => buffer.cursor_right(false),
            "pageup" => self.scroll(-40.0),
            "pagedown" => self.scroll(40.0),
            "return" => buffer.break_line_with_auto_indent(),
            "s-return" => buffer.break_line(),
            "backspace" => buffer.remove_selection(),
            "s-backspace" => buffer.remove_selection(),
            "tab" => buffer.action_insert_text("    ".to_string()),
            "s-up" => buffer.cursor_up(1, true),
            "s-down" => buffer.cursor_down(1, true),
            "s-left" => buffer.cursor_left(true),
            "s-right" => buffer.cursor_right(true),
            "c-a" => self.select_all(buffer),
            "c-c" => buffer.clipboard_copy(),
            "c-s" => buffer.save(),
            "c-v" => buffer.clipboard_paste(),
            "c-x" => buffer.clipboard_cut(),
            "c-z" => buffer.undo(),
            "c-up" => buffer.cursor_up(1, false),
            "c-down" => buffer.cursor_down(1, false),
            "c-s-up" => buffer.cursor_up(1, true),
            "c-s-down" => buffer.cursor_down(1, true),
            "c-right" => {
                let (x, y) = buffer.next_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.set_selection(false);
            }
            "c-left" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.set_selection(false);
            }
            "c-s-right" => {
                let (x, y) = buffer.next_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
            }
            "c-s-left" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
            }
            "c-s-z" => buffer.redo(),
            "c-backspace" => {
                let (x, y) = buffer.prev_word(buffer.cursor_x, buffer.cursor_y);
                buffer.cursor_x = x;
                buffer.cursor_y = y;
                buffer.remove_selection();
            }
            "c-s-\\" => {
                buffer.print();
                return true;
            }
            _ => {}
        }
        false
    }

    pub fn scroll(&mut self, lines: f32) {
        let padding = 5.0;
        let bar_height: f32 = self.line_height + padding * 2.0;

        let mut new_value = self.scroll_lag + lines * self.line_height;
        let new_offset = (self.scroll_offset as f32 + new_value) / self.line_height;

        let scrolloff_start = 0.0;
        let scrolloff_end = 5.0;
        let end_val = f32::max(0.0, self.display_line_count as f32 + scrolloff_end - (self.rect.height - bar_height) / self.line_height);
        if new_offset < -scrolloff_start {
            new_value = -scrolloff_start * self.line_height - self.scroll_offset as f32;
        } else if new_offset > end_val {
            new_value = end_val * self.line_height - self.scroll_offset as f32;
        }

        self.scroll_lag = new_value;
    }

    pub fn select_all(&mut self, buffer: &mut Buffer) {
        // TODO scroll to end of file?
        buffer.select_all();
    }

    fn get_focused_cell(
        &mut self,
        app: &Engine,
        x: f32,
        y: f32,
    ) -> (usize, usize) {
        let padding = 5.0;
        let bar_height = app.font_size + padding * 2.0;
        let x_cell = ((x - padding * 2.0 - (app.char_width / 2.0)) / app.char_width) as usize;
        let y_cell = ((y - padding - bar_height - (app.font_size / 2.0)) / app.font_size)
            as usize;
        (x_cell, y_cell)
    }

    // Set the selection/cursor positions based on screen coordinates.
    // TODO This doesn't actually take screen coordinates because draw() handles that now - What is
    // this function actually doing? Do I need to port this logic into draw()?
    pub fn set_selection_from_screen(
        &mut self,
        // app: &Engine,
        mut buffer: &mut Buffer,
        // mouse_x: f32,
        // mouse_y: f32,
        extend: bool,
    ) {
        let mut x_target = 0;
        let mut y_target = 0;
        let line_lengths = buffer.contents.iter().map(|line| line.as_str().graphemes(true).collect::<Vec<&str>>().len() + 1).collect::<Vec<usize>>();
        let line_count = line_lengths.len();
        if self.cursor_y > 0 && self.cursor_y < line_count {
            y_target = self.cursor_y;
        } else if self.cursor_y >= line_count {
            y_target = line_count - 1;
        }

        let char_count = line_lengths[y_target];
        if self.cursor_x > 0 && self.cursor_x < char_count {
            x_target = self.cursor_x;
        } else if self.cursor_x >= char_count {
            x_target = char_count - 1;
        }

        buffer.cursor_x = x_target;
        buffer.cursor_y = y_target;
        buffer.set_selection(extend);
    }
}
