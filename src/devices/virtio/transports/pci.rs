use crate::{
    devices::virtio::virtio::VirtioDevice,
    pci::{config_instance::ConfigInstance, device::PCIDevice},
};

use std::ops::RangeInclusive;

pub struct PCITransport {
    config: ConfigInstance,
}

impl PCITransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize, irq_sel: u32) -> Self {
        let mut config = ConfigInstance::new();
        config.add_config_region(vec![0x1A, 0xFA], 0x0, false); // Vendor

        Self { config }
    }
}

impl PCIDevice for PCITransport {
    fn read_config(&self, function: &u8, dword: &u8, dword_offset: &u8) -> Option<u8> {
        Some(0)
    }

    fn write_config(&mut self, function: &u8, dword: &u8, dword_offset: &u8, data: u8) {}

    fn mmio_read(&self, addr: u64) -> Option<u8> {
        Some(0)
    }

    fn mmio_write(&mut self, addr: u64, value: u8) {}

    fn get_mmio_range(&self) -> RangeInclusive<u64> {
        0u64..=0u64
    }
}
