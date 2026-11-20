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

    pub fn read_config(&self, device: u8, function: u8, dword: u8, dword_select: u8) -> Option<u8> {
        if let Some(device) = self.devices.get(device as usize) {
            device.read_config(&function, &dword, &dword_select)
        } else {
            Some(0xFF)
        }
    }

    pub fn write_config(
        &mut self,
        device: u8,
        function: u8,
        dword: u8,
        dword_select: u8,
        byte: u8,
    ) {
        if let Some(device) = self.devices.get_mut(device as usize) {
            device.write_config(&function, &dword, &dword_select, byte);
        }
    }

    pub fn write_mmio(&mut self, addr: u64, byte: u8) {
        for i in 0..self.devices.len() {
            if let Some(device) = self.devices.get_mut(i) {
                let range = device.get_mmio_range();
                if range.contains(&addr) {
                    device.mmio_write(addr - range.start(), byte);
                }
            }
        }
    }

    pub fn read_mmmio(&self, addr: u64) -> Option<u8> {
        for i in 0..self.devices.len() {
            if let Some(device) = self.devices.get(i) {
                let range = device.get_mmio_range();
                if range.contains(&addr) {
                    return Some(device.mmio_read(addr - range.start())?);
                }
            }
        }
        return None;
    }
}
