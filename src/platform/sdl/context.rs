extern crate sdl2;

use sdl2::{pixels::Color, render::Canvas, video::Window};

use crate::platform::display::DisplayBackend;

pub struct FerrumSDLContext {
    canvas: Canvas<Window>,
}

// SDL's Canvas wraps Rc internally, so it is not Send by default.
// The VMM drives this device from a single thread, so this is safe.
unsafe impl Send for FerrumSDLContext {}

impl FerrumSDLContext {
    pub fn new(width: u32, height: u32) -> Option<Self> {
        let sdl_context = sdl2::init().ok()?;
        let video_subsystem = sdl_context.video().ok()?;

        let window = video_subsystem
            .window("Ferrum VM", width, height)
            .position_centered()
            .build()
            .ok()?;
        let mut canvas = window.into_canvas().build().ok()?;
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();
        canvas.present();
        Some(Self { canvas })
    }

    pub fn update(&mut self) {
        self.canvas.present();
    }
}

impl DisplayBackend for FerrumSDLContext {
    fn resize_display(&mut self, width: u32, height: u32) -> bool {
        let window = self.canvas.window_mut();
        if let Err(_) = window.set_size(width, height) {
            false
        } else {
            true
        }
    }

    fn get_display_size(&self) -> (u32, u32) {
        self.canvas.window().size()
    }

    fn upload(&mut self, _framebuffer: &[u8], _width: u32, _height: u32, _stride: u32) {}
    fn present(&mut self) {}
}
