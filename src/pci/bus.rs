use crate::pci::device::PCIDevice;

pub struct PCIBus {
    devices: Vec<Box<dyn PCIDevice>>,
}

impl PCIBus {
    pub fn new() -> Self {
        Self { devices: vec![] }
    }

    pub fn register(&mut self, device: Box<dyn PCIDevice>) {
        self.devices.push(device);
    }
}
