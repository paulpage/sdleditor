use sdl2::{Sdl, VideoSubsystem};
use sdl2::rect::Rect as SdlRect;
use sdl2::pixels::PixelFormatEnum;
use sdl2::surface::Surface;
use sdl2::render::{Texture, TextureQuery, WindowCanvas};
use std::cmp::{max, min};
use std::collections::HashMap;
use std::rc::Rc;
use std::path::Path;

use rusttype::{point, Font, Scale, PositionedGlyph};

pub use sdl2::pixels::Color;

#[derive(Copy, Clone)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub w: i32,
    pub h: i32,
}

impl Rect {
    pub fn new(x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            x,
            y,
            w,
            h,
        }
    }
}

#[derive(Hash, PartialEq)]
struct FontCacheKey {
    c: String,
    color: Color,
}

struct FontCacheEntry {
    texture: Texture,
    w: i32,
    h: i32,
}

impl Eq for FontCacheKey {}

pub struct Canvas<'a> {
    pub char_width: i32,
    pub font_size: i32,
    rect: Rect,
    pub font: Font<'a>,
    font_cache: HashMap<FontCacheKey, Rc<FontCacheEntry>>,
    canvas: WindowCanvas,
}

impl<'a> Canvas<'a> {

    pub fn new(sdl_context: &mut Sdl, font_path: &Path, font_size: u16) -> Self {
        let video_subsys = sdl_context.video().unwrap();
        let window = video_subsys
            .window("SDL2_TTF Example", 800, 600)
            .position_centered()
            .resizable()
            .maximized()
            .opengl()
            .build()
            .unwrap();
        let canvas: WindowCanvas = window.into_canvas().build().unwrap();

        let font = {
            let data = std::fs::read(font_path).unwrap();
            Font::try_from_vec(data).unwrap()
        };

        let char_width = font.glyph('o').scaled(Scale::uniform(font_size as f32)).h_metrics().advance_width as i32;

        Self {
            char_width,
            font_size: font_size as i32,
            rect: Rect::new(0, 0, 0, 0),
            font,
            font_cache: HashMap::new(),
            canvas,
        }
    }

    pub fn clear(&mut self, color: Color) {
        self.canvas.set_draw_color(color);
        let rect = SdlRect::new(self.rect.x, self.rect.y, self.rect.w as u32, self.rect.h as u32);
        self.canvas.fill_rect(rect).unwrap();
    }

    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.canvas.set_draw_color(color);
        let x = min(self.rect.x + self.rect.w, max(self.rect.x, self.rect.x + rect.x));
        let y = min(self.rect.y + self.rect.h, max(self.rect.y, self.rect.y + rect.y));
        let w = max(0, min(self.rect.w - rect.x, rect.w + min(0, rect.x))) as u32;
        let h = max(0, min(self.rect.h - rect.y, rect.h + min(0, rect.y))) as u32;
        if w > 0 && h > 0 {
            self.canvas.fill_rect(SdlRect::new(x, y, w, h)).unwrap();
        }
    }

    pub fn fill_rect_with_border(&mut self, outer_rect: Rect, border_size: i32, color: Color, border_color: Color) {
        let inner_rect = Rect {
            x: outer_rect.x + border_size,
            y: outer_rect.y + border_size,
            w: outer_rect.w - border_size * 2,
            h: outer_rect.h - border_size * 2,
        };
        self.fill_rect(outer_rect, border_color);
        self.fill_rect(inner_rect, color);
    }

    pub fn layout_text(&self, text: &str, scale: f32) -> (Vec<PositionedGlyph<'_>>, usize, usize) {
        let font_scale = Scale::uniform(scale);
        let v_metrics = self.font.v_metrics(font_scale);
        let glyphs: Vec<_> = self.font
            .layout(text, font_scale, point(0.0, 0.0 + v_metrics.ascent))
            .collect();

        let height = (v_metrics.ascent - v_metrics.descent).ceil() as usize;
        let width = glyphs
            .iter()
            .rev()
            .map(|g| g.position().x as f32 + g.unpositioned().h_metrics().advance_width)
            .next()
            .unwrap_or(0.0)
            .ceil() as usize;
        (glyphs, width, height)
    }

    pub fn draw_char(&mut self, color: Color, x: i32, y: i32, c: &str) {
        self.canvas.set_draw_color(color);
        let key = FontCacheKey {
            c: c.to_string(),
            color,
        };
        let tex = self.font_cache.get(&key).cloned().unwrap_or_else(|| {
            let scale = self.font_size as f32;
            let (glyphs, w, h) = self.layout_text(c, scale);
            let mut buffer: Vec<u8> = vec![0; w * h * 4];
            for glyph in glyphs {
                if let Some(bounding_box) = glyph.pixel_bounding_box() {
                    let min_x = bounding_box.min.x;
                    let min_y = bounding_box.min.y;

                    glyph.draw(|x, y, v| {
                        let x = std::cmp::max(x as i32 + min_x, 1) as usize - 1;
                        let y = std::cmp::max(y as i32 + min_y, 1) as usize - 1;
                        let index = (y * w + x) * 4;
                        buffer[index + 0] = color.r;
                        buffer[index + 1] = color.g;
                        buffer[index + 2] = color.b;
                        buffer[index + 3] = (v * 255.0) as u8;
                    });
                }
            }
            let surface = Surface::from_data(&mut buffer, w as u32, h as u32, 4 * w as u32, PixelFormatEnum::ABGR8888).unwrap();

            let texture = self.canvas
                .texture_creator()
                .create_texture_from_surface(&surface)
                .unwrap();
            let TextureQuery { width, height, .. } = texture.query();
            let resource = Rc::new(FontCacheEntry {
                texture,
                w: width as i32,
                h: height as i32,
            });
            self.font_cache.insert(key, resource.clone());
            resource
        });
        let texture = &tex.texture;
        let w = min(self.rect.w - (x + self.char_width), tex.w as i32) as u32;
        let h = min(self.rect.h - y, tex.h as i32) as u32;
        let source = SdlRect::new(0, 0, w, h);
        let target = SdlRect::new(self.rect.x + x, self.rect.y + y, w, h);
        self.canvas.copy(&texture, Some(source), Some(target)).unwrap();
    }

    pub fn text_length(&self, text: &str) -> i32 {
        // TODO make this correct for non-monospace
        text.len() as i32 * self.char_width
        // let mut length = 0;
        // for c in text.chars() {
        //     let (x, _) = self.font.size_of_char(c).unwrap();
        //     length += x as i32;
        // }
        // length
    }

    pub fn set_font(&mut self, path: &Path, size: u16) {
        self.font = {
            let data = std::fs::read(path).unwrap();
            Font::try_from_vec(data).unwrap()
        };
        self.font_size = size as i32;
    }

    pub fn set_active_region(&mut self, rect: Rect) {
        self.rect = rect;
    }

    pub fn window_size(&self) -> (i32, i32) {
        let (w, h) = self.canvas.window().size();
        (w as i32, h as i32)
    }

    pub fn present(&mut self) {
        self.canvas.present();
    }
}
