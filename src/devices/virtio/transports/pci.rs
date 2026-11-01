use crate::{devices::virtio::virtio::VirtioDevice, pci::device::PCIDevice};

pub struct PCITransport {}

impl PCITransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize, irq_sel: u32) -> Self {
        Self {}
    }
}

impl PCIDevice for PCITransport {}
