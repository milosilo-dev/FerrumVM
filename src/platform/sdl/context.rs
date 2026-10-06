extern crate sdl2;

use sdl2::pixels::PixelFormatEnum;
use sdl2::rect::Rect;
use sdl2::render::{Canvas, Texture, TextureCreator};
use sdl2::{pixels::Color, video::Window, video::WindowContext};

use crate::platform::display::DisplayBackend;
use crate::platform::display::DisplayRect;

pub struct FerrumSDLContext {
    canvas: Canvas<Window>,
    texture_creator: &'static TextureCreator<WindowContext>,
    texture: Texture<'static>,
    width: u32,
    height: u32,
}

unsafe impl Send for FerrumSDLContext {}

impl FerrumSDLContext {
    pub fn new(width: u32, height: u32) -> Result<Self, String> {
        let sdl_context = sdl2::init()?;
        let video_subsystem = sdl_context.video()?;

        let window = match video_subsystem
            .window("Ferrum VM", width, height)
            .position_centered()
            .build()
        {
            Ok(w) => w,
            Err(e) => {
                eprintln!("Could not create window: {:?}", e);
                return Err(format!("Could not create window: {:?}", e));
            }
        };

        let mut canvas = window.into_canvas().build()?;
        canvas.set_draw_color(Color::RGB(255, 255, 255));
        canvas.clear();

        let texture_creator: &'static TextureCreator<WindowContext> =
            Box::leak(Box::new(canvas.texture_creator()));

        let texture =
            texture_creator.create_texture_streaming(PixelFormatEnum::RGBA8888, width, height)?;

        Ok(Self {
            canvas,
            texture_creator,
            texture,
            width,
            height,
        })
    }

    fn blit_rect(
        &mut self,
        src: &[u8],
        src_stride: usize,
        src_x: u32,
        src_y: u32,
        w: u32,
        h: u32,
        dst_x: u32,
        dst_y: u32,
    ) {
        let bpp = 4;
        let row_len = w as usize * bpp;

        let mut rows = vec![0u8; row_len * h as usize];
        for row in 0..h as usize {
            let src_start = (src_y as usize + row) * src_stride + src_x as usize * bpp;
            if src_start >= src.len() {
                break;
            }

            let len = row_len.min(src.len() - src_start);
            let dst_start = row * row_len;
            rows[dst_start..dst_start + len].copy_from_slice(&src[src_start..src_start + len]);
        }

        let w = w.min(self.width.saturating_sub(dst_x));
        let h = h.min(self.height.saturating_sub(dst_y));
        if w == 0 || h == 0 {
            return;
        }

        let rect = Rect::new(dst_x as i32, dst_y as i32, w, h);
        if let Err(e) = self.texture.update(Some(rect), &rows, row_len) {
            eprintln!("virtio-gpu: texture update failed: {}", e);
            return;
        }

        if let Err(e) = self.canvas.copy(&self.texture, None, None) {
            eprintln!("virtio-gpu: render copy failed: {}", e);
        }
    }
}

impl DisplayBackend for FerrumSDLContext {
    fn resize_display(&mut self, width: u32, height: u32) -> bool {
        let window = self.canvas.window_mut();
        if let Err(e) = window.set_size(width, height) {
            eprintln!("SDL resize failed: {}", e);
            false
        } else {
            self.width = width;
            self.height = height;
            match self.texture_creator.create_texture_streaming(
                PixelFormatEnum::RGBA8888,
                width,
                height,
            ) {
                Ok(tex) => {
                    self.texture = tex;
                    true
                }
                Err(e) => {
                    eprintln!("SDL texture recreation failed: {}", e);
                    false
                }
            }
        }
    }

    fn get_display_size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn upload(&mut self, _framebuffer: &[u8], _width: u32, _height: u32, _stride: u32) {}

    fn blit(&mut self, src: &[u8], src_stride: usize, src_rect: DisplayRect) {
        // src_rect.x/y are resource-relative; for a scanout at (0,0), dst = src
        self.blit_rect(
            src,
            src_stride,
            src_rect.x,
            src_rect.y,
            src_rect.width,
            src_rect.height,
            src_rect.x,
            src_rect.y, // Adjust this if scanout has offset!
        );
        self.present();
    }

    fn present(&mut self) {
        self.canvas.present();
    }
}
