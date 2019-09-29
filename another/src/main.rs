extern crate sdl2;

use std::cmp::{max, min};
use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::time::{Instant, Duration};
use std::collections::HashMap;

use sdl2::event::Event;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::pixels::Color;
use sdl2::rect::Rect;
use sdl2::render::{Texture, TextureQuery, TextureCreator, WindowCanvas};
use sdl2::ttf::Font;
use sdl2::video::WindowContext;

fn main() -> Result<(), String> {
    let sdl_context = sdl2::init()?;
    let video_subsys = sdl_context.video()?;
    let ttf_context = sdl2::ttf::init().map_err(|e| e.to_string())?;
    let window = video_subsys
        .window("SDL2_TTF Example", 800, 600)
        .position_centered()
        .opengl()
        .build()
        .map_err(|e| e.to_string())?;
    let mut canvas = window.into_canvas().build().map_err(|e| e.to_string())?;

    let mut panes: Vec<Pane> = Vec::new();
    'mainloop: loop {
        for event in sdl_context.event_pump()?.poll_iter() {
            match event {
                Event::Quit { .. } => break 'mainloop,
                _ => {
                    let pane = &mut app.panes[app.pane_idx];
                    match pane.pane_type {
                        PaneType::Buffer => {
                            if let Some(buffer) = pane.buffer_id {
                                let buffer = &mut app.buffers[buffer];
                                pane.handle_buffer_event(buffer, event);
                            }
                        }
                        PaneType::FileManager => {}
                    }
                }
            }
        }

        canvas.set_draw_color(Color::RGBA(200, 200, 200, 255));
        canvas.clear();

        // DRAW

        sleep(Duration::from_millis(10));
        canvas.present()
    }
}
