extern crate sdl2;

use sdl2::pixels::Color;
use sdl2::render::{Texture, TextureQuery, TextureCreator, WindowCanvas};
use sdl2::ttf::Font;
use sdl2::video::WindowContext;

struct Buffer {
    name: String,
    contents: Vec<String>,
}

struct Pane {
    x: i32,
    y: i32,
    w: u32,
    h: u32,
    buffer_id: Option<usize>,
    cursor_x: usize,
    max_cursor_x: usize,
    cursor_y: usize,
    scroll_idx: usize,
    scroll_offset: i32,
}

fn draw_text(canvas: &mut WindowCanvas, pane: &mut Pane, texture_creator: &TextureCreator, glyph_cache: HashMap<char, Texture>, font: &mut Font, color: Color, x: i32, y: i32, text: &str) {
        if y > 0 && x > 0 {
            let mut length: i32 = 0;
            for c in text.chars() {
                let surface = font
                    .render(&c.to_string())
                    .blended(color)
                    .unwrap();
                // let texture_creator = canvas.texture_creator();
                let texture = texture_creator
                    .create_texture_from_surface(&surface)
                    .unwrap();
                let TextureQuery {
                    width: w,
                    height: h,
                    ..
                } = texture.query();
                let w = min(pane.w as i32 - (x + length as i32), w as i32);
                let h = min(pane.y as i32 - y, h as i32);
                let source = rect!(0, 0, w, h);
                let target = rect!(pane.x + x + length as i32, pane.y + y, w, h);
                canvas.copy(&texture, Some(source), Some(target)).unwrap();

                if length > pane.w as i32 {
                    return;
                }
                length += w;
                if !glyph_cache.contains_key(&c) {
                    glyph_cache.insert(c, texture);
                }
            }
        }
    }


fn main() {
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().unwrap();

    let buffers: Vec<Buffer> = Vec::new();
    let panes: Vec<Pane> = Vec::new();

    let font = ttf_context.load_font("data/LiberationSans-Regular.ttf", 16);
    let texture_creator = canvas.texture_creator();
    let glyph_cache: HashMap<char, Texture> = HashMap::new();
}
