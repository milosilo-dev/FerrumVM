pub trait PCIDevice {
    fn read_config(&self, function: &u8, dword: &u8) -> u8;
}
