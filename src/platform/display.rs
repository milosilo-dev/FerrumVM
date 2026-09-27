pub trait DisplayBackend {
    /// Change the size of this backend
    fn resize_display(&mut self, width: u32, height: u32) -> bool;
    /// Get the size of this backend
    fn get_display_size(&self) -> (u32, u32);

    /// Upload changes to the framebuffer pixles
    fn upload(&mut self, framebuffer: &[u8], width: u32, height: u32, stride: u32);
    /// Draw to back buffer
    fn blit(&mut self, src: &[u8], src_stride: usize, src_rect: DisplayRect);
    /// Present the changes to the screen
    fn present(&mut self);
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub struct DisplayRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl DisplayRect {
    pub fn new(x: u32, y: u32, width: u32, height: u32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}
