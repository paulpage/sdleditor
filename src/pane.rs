use std::cmp::{max, min};
use sdl2::render::{Texture, TextureQuery, TextureCreator, WindowCanvas};
use sdl2::ttf::Font;
use sdl2::video::WindowContext;
use sdl2::rect::Rect;
use sdl2::keyboard::{Keycode, Mod};
use sdl2::event::Event;
use sdl2::pixels::Color;

pub enum PaneType {
    Buffer,
    FileManager,
}
