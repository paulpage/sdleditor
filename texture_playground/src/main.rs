extern crate sdl2;

use std::collections::HashMap;
use std::cmp::{min, max};
use std::thread::sleep;
use std::time::{Instant, Duration};

use sdl2::event::Event;
use sdl2::pixels::Color;
use sdl2::render::{Texture, TextureQuery, TextureCreator, WindowCanvas};
use sdl2::ttf::Font;
use sdl2::video::WindowContext;
use sdl2::rect::Rect;

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

fn draw_text<'a>(canvas: &mut WindowCanvas, pane: &mut Pane, texture_creator: &'a TextureCreator<WindowContext>, glyph_cache: &mut HashMap<char, Texture<'a>>, font: &Font, color: Color, x: i32, y: i32, text: &str) {
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
                let source = Rect::new(0, 0, w as u32, h as u32);
                let target = Rect::new(pane.x + x + length as i32, pane.y + y, w as u32, h as u32);
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
    let sdl_context = sdl2::init().unwrap();
    let video_subsys = sdl_context.video().unwrap();
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string()).unwrap();
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string()).unwrap();
    let mut canvas = window.into_canvas().build().unwrap();

    let mut buffers: Vec<Buffer> = Vec::new();
    let mut panes: Vec<Pane> = Vec::new();

    buffers.push(Buffer {
        name: "TEST".to_string(),
        contents: Vec::new(),
    });
    panes.push(Pane {
        x: 100,
        y: 100,
        w: 400,
        h: 400,
        buffer_id: Some(0),
        cursor_x: 0,
        max_cursor_x: 0,
        cursor_y: 0,
        scroll_idx: 0,
        scroll_offset: 0,
    });

    let font = ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap();
    let texture_creator = canvas.texture_creator();
    let mut glyph_cache: HashMap<char, Texture> = HashMap::new();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGBA(40, 0, 0, 255));
        canvas.clear();

        draw_text(
            &mut canvas,
            &mut panes[0],
            &texture_creator,
            &mut glyph_cache,
            &font,
            Color::RGBA(255, 255, 255, 255),
            100,
            100,
            "Hello, World!");

        sleep(Duration::from_millis(10));
        canvas.present()
    }
}
