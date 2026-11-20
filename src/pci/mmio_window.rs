use crate::device_maps::mmio::MMIODevice;
use crate::pci::bus::PCIBus;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::Mutex;

pub struct PCIMMIOWindow {
    bus: Arc<Mutex<PCIBus>>,
    pci_range: RangeInclusive<u64>,
}

impl PCIMMIOWindow {
    pub fn new(bus: Arc<Mutex<PCIBus>>, pci_range: RangeInclusive<u64>) -> Self {
        Self { bus, pci_range }
    }
}

impl MMIODevice for PCIMMIOWindow {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        if let Ok(bus) = self.bus.lock() {
            let mut res = vec![];
            for i in 0..length {
                res.push(bus.read_mmmio(addr + i as u64).ok_or(0).unwrap())
            }
            return res;
        }
        vec![]
    }

    fn write(&mut self, addr: u64, data: &[u8]) {
        if let Ok(mut bus) = self.bus.lock() {
            for i in 0..data.len() {
                if let Some(data_byte) = data.get(i) {
                    bus.write_mmio(addr + i as u64, *data_byte);
                }
            }
        }
    }
}
