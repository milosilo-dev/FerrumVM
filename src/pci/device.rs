pub trait PCIDevice: Send {
    fn read_config(&self, function: &u8, dword: &u8, dword_offset: &u8) -> Option<u8>;
    fn write_config(&mut self, function: &u8, dword: &u8, dword_offset: &u8, data: u8);
}
