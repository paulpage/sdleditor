extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::collections::HashMap;
use std::time::Duration;
use std::rc::Rc;

use sdl2::event::Event;
// use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, TextureCreator, WindowCanvas};
use sdl2::ttf::{Font, Sdl2TtfContext};
use sdl2::video::WindowContext;

struct Buffer {
    name: String,
    contents: Vec<String>,
    is_dirty: bool,
}

// ========================================

// type TextureManager<'l, T> = ResourceManager<'l, String, Texture<'l>, TextureCreator<T>>;
// type FontManager<'l> = ResourceManager<'l, FontDetails, Font<'l, 'static>, Sdl2TtfContext>;


// // Generic struct to cache any resource loaded by a ResourceLoader
// pub struct ResourceManager<'l, K, R, L>
//     where K: Hash + Eq,
//           L: 'l + ResourceLoader<'l, R>
// {
//     loader: &'l L,
//     cache: HashMap<K, Rc<R>>,
// }

// impl<'l, K, R, L> ResourceManager<'l, K, R, L>
//     where K: Hash + Eq,
//           L: ResourceLoader<'l, R>
// {
//     pub fn new(loader: &'l L) -> Self {
//         ResourceManager {
//             cache: HashMap::new(),
//             loader: loader,
//         }
//     }

//     // Generics magic to allow a HashMap to use String as a key
//     // while allowing it to use &str for gets
//     pub fn load<D>(&mut self, details: &D) -> Result<Rc<R>, String>
//         where L: ResourceLoader<'l, R, Args = D>,
//               D: Eq + Hash + ?Sized,
//               K: Borrow<D> + for<'a> From<&'a D>
//     {
//         self.cache
//             .get(details)
//             .cloned()
//             .map_or_else(|| {
//                              let resource = Rc::new(self.loader.load(details)?);
//                              self.cache.insert(details.into(), resource.clone());
//                              Ok(resource)
//                          },
//                          Ok)
//     }
// }

// // TextureCreator knows how to load Textures
// impl<'l, T> ResourceLoader<'l, Texture<'l>> for TextureCreator<T> {
//     type Args = str;
//     fn load(&'l self, path: &str) -> Result<Texture, String> {
//         println!("LOADED A TEXTURE");
//         self.load_texture(path)
//     }
// }

// // Font Context knows how to load Fonts
// impl<'l> ResourceLoader<'l, Font<'l, 'static>> for Sdl2TtfContext {
//     type Args = FontDetails;
//     fn load(&'l self, details: &FontDetails) -> Result<Font<'l, 'static>, String> {
//         println!("LOADED A FONT");
//         self.load_font(&details.path, details.size)
//     }
// }

// // Generic trait to Load any Resource Kind
// pub trait ResourceLoader<'l, R> {
//     type Args: ?Sized;
//     fn load(&'l self, data: &Self::Args) -> Result<R, String>;
// }

// // Information needed to load a Font
// #[derive(PartialEq, Eq, Hash)]
// pub struct FontDetails {
//     pub path: String,
//     pub size: u16,
// }

// impl<'a> From<&'a FontDetails> for FontDetails {
//     fn from(details: &'a FontDetails) -> FontDetails {
//         FontDetails {
//             path: details.path.clone(),
//             size: details.size,
//         }
//     }
// }

// ========================================

struct FontManager<'a> {
    font: Font<'a, 'static>,
    color: Color,
    texture_creator: &'a TextureCreator<WindowContext>,
    cache: HashMap<char, Rc<Texture>>,
}

impl<'a> FontManager<'a> {
    fn new(texture_creator: &'a TextureCreator<WindowContext>, ttf_context: &'a Sdl2TtfContext, font_path: &str, font_size: u16, font_color: Color) -> FontManager<'a> {
        let font: Font<'a, 'static> = ttf_context.load_font(font_path, font_size).unwrap();
        FontManager {
            font: font,
            color: font_color,
            texture_creator: texture_creator,
            cache: HashMap::new(),
        }
    }

    fn load(&mut self, key: char) -> Result<Rc<Texture>, String> {
        self.cache
            .get(&key)
            .cloned()
            .map_or_else(|| {
                let surface = self.font
                    .render(&key.to_string())
                    .blended(self.color)
                    .unwrap();
                let texture: Texture = self.texture_creator
                    .create_texture_from_surface(&surface)
                    .unwrap();
                let resource = Rc::new(texture);
                self.cache.insert(key, resource.clone());
                Ok(resource)
            },
            Ok)
    }
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
    line_height: i32,
}

fn fill_rect(pane: &mut Pane, canvas: &mut WindowCanvas, color: Color, rect: Rect) {
    canvas.set_draw_color(color);
    let x = pane.x + max(rect.x, 0);
    let y = pane.y + max(rect.y, 0);
    let w = min(pane.w as i32 - rect.x, rect.w) as u32;
    let h = min(pane.h as i32 - rect.y, rect.h) as u32;
    if w > 0 && h > 0 {
        canvas.fill_rect(Rect::new(x, y, w, h as u32)).unwrap();
    }
}

impl<'a> Pane {

    fn draw(&self, canvas: &mut WindowCanvas) {
        let rect = Rect::new(self.x, self.y, self.w, self.h);
        canvas.fill_rect(rect).unwrap();
    }
    fn draw_text(&mut self, canvas: &mut WindowCanvas, fm: &'a mut FontManager<'a>, x: i32, y: i32, text: &str) {
        if y > 0 && x > 0 {
            let mut length: i32 = 0;
            for c in text.chars() {
                let texture = fm.load(c).unwrap();
                // if let Some(texture) = fm.cache.get(&c) {
                    let TextureQuery {
                        width: mut w,
                        height: mut h,
                        ..
                    } = texture.query();
                    w = min(self.w as i32 - (x as i32 + length as i32), w as i32) as u32;
                    h = min(self.h as i32 - y as i32, h as i32) as u32;
                    let source = Rect::new(0, 0, w as u32, h as u32);
                    let target = Rect::new(self.x + x + length as i32, self.y + y, w as u32, h as u32);
                    canvas.copy(&texture, Some(source), Some(target)).unwrap();

                    if length > self.w as i32 {
                        return;
                    }
                    length += w as i32;
                // } else {
                //     let surface = fm.font
                //         .render(&c.to_string())
                //         .blended(fm.color)
                //         .unwrap();
                //     let texture: Texture<'a> = fm.texture_creator
                //         .create_texture_from_surface(&surface)
                //         .unwrap();
                //     let TextureQuery {
                //         width: mut w,
                //         height: mut h,
                //         ..
                //     } = &texture.query();
                //     w = min(self.w as i32 - (x as i32 + length as i32), w as i32) as u32;
                //     h = min(self.h as i32 - y as i32, h as i32) as u32;
                //     let source = Rect::new(0, 0, w as u32, h as u32);
                //     let target = Rect::new(self.x + x + length as i32, self.y + y, w as u32, h as u32);
                //     canvas.copy(&texture, Some(source), Some(target)).unwrap();

                //     if length > self.w as i32 {
                //         return;
                //     }
                //     length += w as i32;
                //     fm.cache.entry(c).or_insert(texture);
                // }
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
    let mut canvas: WindowCanvas = window.into_canvas().build().unwrap();
    let texture_creator = canvas.texture_creator();
    // let font = ttf_context.load_font("data/LiberationSans-Regular.ttf", 16).unwrap();
    let font_manager = FontManager::new(&texture_creator, &ttf_context, "data/LiberationSans-Regular.ttf", 16, Color::RGBA(255, 255, 255, 255));
    // let font_manager = FontManager {
    //     font: font,
    //     color: Color::RGBA(255, 255, 255, 255),
    //     texture_creator: &texture_creator,
    //     cache: HashMap::new(),
    // };


    let mut buffers: Vec<Buffer> = Vec::new();
    let mut panes: Vec<Pane> = Vec::new();
    let mut buffer_idx = 0;
    let mut pane_idx = 0;

    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            buffers.push(Buffer {
                contents: Vec::new(),
                name: args[1].clone(),
                is_dirty: false,
            });
            let file = fs::File::open(&args[1]).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                buffers[buffer_idx].contents.push(line.unwrap());
            }
        }
        _ => {
            buffers.push(Buffer {
                contents: Vec::new(),
                name: "UNNAMED".to_string(),
                is_dirty: false,
            });
            buffers[buffer_idx].contents.push(String::new());
        }
    }

    let (_width, height) = canvas.window().size();
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
        line_height: 0,
    });
    panes[pane_idx].line_height = font_manager.font.height();

    'mainloop: loop {
        for event in sdl_context.event_pump().unwrap().poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                _ => {}
            }
        }

        canvas.set_draw_color(Color::RGBA(40, 0, 0, 255));
        canvas.clear();

        canvas.set_draw_color(Color::RGBA(100, 100, 30, 255));
        // panes[pane_idx].draw(&mut canvas);
        // let pane_rect = Rect::new(panes[pane_idx].x, panes[pane_idx].y, panes[pane_idx].w, panes[pane_idx].h);
        // canvas.fill_rect(pane_rect).unwrap();

        // for i in 0..s  {
            // let pane = &mut panes[i];
        for mut pane in &mut panes {
            pane.draw(&mut canvas);
            let line_height = pane.line_height;
            // Smooth scrolling
            let target_scroll_offset = pane.scroll_idx as i32 * line_height as i32;
            let scroll_delta = target_scroll_offset - pane.scroll_offset;
            if pane.scroll_offset < target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).ceil() as i32;
            } else if pane.scroll_offset > target_scroll_offset {
                pane.scroll_offset += (f64::from(scroll_delta) / 3.0).floor() as i32;
            }
            let scroll_offset = pane.scroll_offset;

            if let Some(b) = pane.buffer_id {
                let buffer = &buffers[b];
                canvas.set_draw_color(Color::RGBA(150, 0, 150, 255));
                let rect = Rect::new(pane.x, pane.y, pane.w, pane.h);
                canvas.fill_rect(rect).unwrap();

                // We only want to render the lines that are actually on the screen.
                let first_line = max(
                    0,
                    pane.scroll_idx as i32
                    - (f64::from(height) / f64::from(line_height)).ceil() as i32,
                    ) as usize;
                let last_line = min(
                    buffer.contents.len(),
                    pane.scroll_idx
                    + (f64::from(height) / f64::from(line_height)).ceil() as usize,
                    );

                let padding: u32 = 5;
                let bar_height: u32 = line_height as u32 + padding * 2;
                // Draw the contents of the file and the cursor.

                for (i, entry) in buffer.contents[first_line..last_line].iter().enumerate() {

                    // Render the full line of text
                    pane.draw_text(
                        &mut canvas,
                        &mut font_manager,
                        // &mut pane,
                        // &texture_creator,
                        // &mut glyph_cache,
                        // &mut text_font,
                        // Color::RGBA(40, 0, 0, 255),
                        padding as i32,
                        bar_height as i32 + padding as i32 + (i as i32 + first_line as i32) * line_height as i32
                        - scroll_offset,
                        &entry,
                        );
                }
            }
        }

        sleep(Duration::from_millis(5));
        canvas.present(); 
    }
}
