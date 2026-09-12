pub trait DisplayBackend {
    fn resize_display(&mut self, width: u32, height: u32) -> bool;
    fn get_display_size(&self) -> (u32, u32);

    fn upload(&mut self, framebuffer: &[u8], width: u32, height: u32, stride: u32);
    fn present(&mut self);
}
