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

    pub fn read_config(&self, device: u8, function: u8, dword: u8) -> Option<u8> {
        if let Some(device) = self.devices.get(device as usize) {
            Some(device.read_config(&function, &dword))
        } else {
            None
        }
    }
}
