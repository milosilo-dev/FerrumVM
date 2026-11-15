use crate::{devices::virtio::virtio::VirtioDevice, pci::device::PCIDevice};

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
