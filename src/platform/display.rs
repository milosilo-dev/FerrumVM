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
#[derive(Debug, Copy, Clone)]
pub struct DisplayRect {
    pub(crate) x: u32,
    pub(crate) y: u32,
    pub(crate) width: u32,
    pub(crate) height: u32,
}

impl DisplayRect {
    pub(crate) fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }
}
