use crate::device_maps::mmio::MMIODevice;
use crate::pci::bus::PCIBus;
use std::ops::RangeInclusive;
use std::sync::Arc;
use std::sync::Mutex;

pub struct PCIMMIOWindow {
    buses: Arc<Mutex<Vec<PCIBus>>>,
    pci_range: RangeInclusive<u64>,
}

impl PCIMMIOWindow {
    pub fn new(buses: Arc<Mutex<Vec<PCIBus>>>, pci_range: RangeInclusive<u64>) -> Self {
        Self { buses, pci_range }
    }
}

impl MMIODevice for PCIMMIOWindow {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        vec![]
    }

    fn write(&mut self, addr: u64, data: &[u8]) {}
}
