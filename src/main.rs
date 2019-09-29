extern crate sdl2;

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::thread::sleep;
use std::time::Duration;

use sdl2::event::Event;
use sdl2::pixels::Color;

mod pane;
use pane::{Buffer, Pane, PaneType};

// use crate::pane::{Pane, PaneType};


struct App {
    buffers: Vec<Buffer>,
    panes: Vec<Pane>,
    buffer_idx: usize,
    pane_idx: usize,
}

fn main() -> Result<(), String> {
    // Initialize video subsystem
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

    // Initialize app
    let mut app = App {
        buffers: Vec::new(),
        panes: Vec::new(),
        buffer_idx: 0,
        pane_idx: 0,
        // canvas: canvas,
    };

    let args: Vec<String> = env::args().collect();
    match args.len() {
        2 => {
            app.buffers.push(Buffer {
                contents: Vec::new(),
                name: args[1].clone(),
                is_dirty: false,
            });
            let file = fs::File::open(&args[1]).unwrap();
            let reader = BufReader::new(file);
            for line in reader.lines() {
                app.buffers[app.buffer_idx].contents.push(line.unwrap());
            }
        }
        _ => {
            app.buffers.push(Buffer {
                contents: Vec::new(),
                name: "UNNAMED".to_string(),
                is_dirty: false,
            });
            app.buffers[app.buffer_idx].contents.push(String::new());
        }
    }

    let font = ttf_context
        .load_font("data/LiberationSans-Regular.ttf", 16)
        .unwrap();
    let (width, height) = canvas.window().size();
    let texture_creator = canvas.texture_creator();
    app.panes.push(Pane {
        pane_type: PaneType::Buffer,
        x: 50,
        y: 50,
        w: width - 100,
        h: height - 100,
        cursor_x: 0,
        max_cursor_x: 0,
        cursor_y: 0,
        font: &font,
        line_height: 0,
        buffer_id: Some(0),
        scroll_idx: 0,
        scroll_offset: 0,
        texture_creator: texture_creator,
        // glyph_atlas: HashMap::new(),
    });
    app.panes[app.pane_idx].line_height = app.panes[app.pane_idx].font.height();

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

        for mut pane in &mut app.panes {
            pane.draw_buffer(&app.buffers[app.buffer_idx]);
        }

        sleep(Duration::from_millis(10));
        canvas.present()
    }

    Ok(())
}
