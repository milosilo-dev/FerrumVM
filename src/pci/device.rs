use std::ops::RangeInclusive;

pub trait PCIDevice: Send {
    fn read_config(&self, function: &u8, dword: &u8, dword_offset: &u8) -> Option<u8>;
    fn write_config(&mut self, function: &u8, dword: &u8, dword_offset: &u8, data: u8);
    fn mmio_read(&self, addr: u64) -> Option<u8>;
    fn mmio_write(&mut self, addr: u64, value: u8);
    fn get_mmio_range(&self) -> RangeInclusive<u64>;
}
