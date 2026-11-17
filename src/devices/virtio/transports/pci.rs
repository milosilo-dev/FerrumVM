use crate::{
    device_maps::mmio::MMIODevice, devices::virtio::virtio::VirtioDevice, pci::device::PCIDevice,
};

pub struct PCITransport {}

impl PCITransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize, irq_sel: u32) -> Self {
        Self {}
    }
}

impl PCIDevice for PCITransport {
    fn read_config(&self, function: &u8, dword: &u8, dword_offset: &u8) -> Option<u8> {
        Some(0)
    }

    fn write_config(&mut self, function: &u8, dword: &u8, dword_offset: &u8, data: u8) {}
}

impl MMIODevice for PCITransport {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        vec![]
    }

    fn write(&mut self, addr: u64, data: &[u8]) {}
}
