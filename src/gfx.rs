//! Thin drawing layer over macroquad with integer, top-left based coordinates.

use macroquad::{shapes, text, window};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Color {
    r: u8,
    g: u8,
    b: u8,
    a: u8,
}

impl Color {
    pub const fn new(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self { r, g, b, a }
    }

    pub const BLACK: Color = Color::new(0, 0, 0, 255);
    pub const RAYWHITE: Color = Color::new(245, 245, 245, 255);
    pub const LIGHTGRAY: Color = Color::new(200, 200, 200, 255);
    pub const GRAY: Color = Color::new(130, 130, 130, 255);
    pub const DARKGRAY: Color = Color::new(80, 80, 80, 255);
    pub const GOLD: Color = Color::new(255, 203, 0, 255);

    fn to_mq(self) -> macroquad::color::Color {
        macroquad::color::Color::from_rgba(self.r, self.g, self.b, self.a)
    }
}

/// Zero-sized handle for drawing into the current frame.
pub struct Canvas;

impl Canvas {
    pub fn clear_background(&mut self, color: Color) {
        window::clear_background(color.to_mq());
    }

    pub fn draw_rectangle(&mut self, x: i32, y: i32, w: i32, h: i32, color: Color) {
        shapes::draw_rectangle(x as f32, y as f32, w as f32, h as f32, color.to_mq());
    }

    pub fn draw_line(&mut self, x1: i32, y1: i32, x2: i32, y2: i32, color: Color) {
        shapes::draw_line(
            x1 as f32,
            y1 as f32,
            x2 as f32,
            y2 as f32,
            1.0,
            color.to_mq(),
        );
    }

    pub fn draw_circle(&mut self, x: i32, y: i32, radius: f32, color: Color) {
        shapes::draw_circle(x as f32, y as f32, radius, color.to_mq());
    }

    /// Draws text with `(x, y)` as its top-left corner.
    pub fn draw_text(&mut self, content: &str, x: i32, y: i32, size: i32, color: Color) {
        let size = size.clamp(1, u16::MAX as i32) as u16;
        let metrics = text::measure_text(content, None, size, 1.0);
        text::draw_text(
            content,
            x as f32,
            y as f32 + metrics.offset_y,
            size as f32,
            color.to_mq(),
        );
    }
}
