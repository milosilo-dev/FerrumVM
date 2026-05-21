use crate::device_maps::mmio::MMIODevice;

pub struct PCI{

}

impl PCI {
    pub fn new() -> Self{
        Self {}
    }
}

impl MMIODevice for PCI{
    fn read(&mut self, _addr: u64, length: usize) -> Vec<u8> {
        vec![0xFF; length]
    }

    fn write(&mut self, _addr: u64, _data: &[u8]) {}
}