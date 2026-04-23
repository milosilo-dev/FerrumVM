use std::{
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use crate::{irq::handler::IRQHandler, memory_region::{GuestMemoryHandle, MemoryRegion}};

pub trait MMIODevice: Send {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8>;
    fn write(&mut self, addr: u64, data: &[u8]);
    fn irq_handler(&mut self, _irq_handler: Arc<Mutex<IRQHandler>>) {}
    fn pass_guest_memory(&mut self, _guest_memory: GuestMemoryHandle) {}
    fn tick(&mut self) {}
}

pub struct MMIODeviceRegion {
    mmio_device: Box<dyn MMIODevice>,
    addr_range: RangeInclusive<u64>,
}

impl MMIODeviceRegion {
    pub fn new(range: RangeInclusive<u64>, device: Box<dyn MMIODevice>) -> Self {
        Self {
            mmio_device: device,
            addr_range: range,
        }
    }

    pub fn contains(&self, addr: u64) -> bool {
        self.addr_range.contains(&addr)
    }

    pub fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        self.mmio_device
            .read(addr - *self.addr_range.start(), length)
    }

    pub fn write(&mut self, addr: u64, data: &[u8]) {
        self.mmio_device
            .write(addr - *self.addr_range.start(), data);
    }

    pub fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.mmio_device.irq_handler(irq_handler);
    }

    pub fn tick(&mut self) {
        self.mmio_device.tick();
    }

    pub fn pass_guest_memory(&mut self, guest_memory:Arc<Mutex<Vec<MemoryRegion>>>) {
        self.mmio_device.pass_guest_memory(guest_memory);
    }
}

pub struct MMIODeviceMap {
    devices: Vec<MMIODeviceRegion>,
}

impl MMIODeviceMap {
    pub fn new() -> Self {
        Self { devices: vec![] }
    }

    pub fn register(&mut self, region: MMIODeviceRegion) {
        self.devices.push(region);
    }

    pub fn read(&mut self, addr: u64, length: usize) -> Option<Vec<u8>> {
        for device in &mut self.devices {
            if device.contains(addr) {
                return Some(device.read(addr, length));
            }
        }
        None
    }

    pub fn write(&mut self, addr: u64, data: &[u8]) -> Option<()> {
        for device in &mut self.devices {
            if device.contains(addr) {
                device.write(addr, data);
                return Some(());
            }
        }
        None
    }

    pub fn tick(&mut self) {
        for device in &mut self.devices {
            device.tick();
        }
    }

    pub fn pass_guest_memory(&mut self, guest_memory: Arc<Mutex<Vec<MemoryRegion>>>) {
        for device in &mut self.devices {
            device.pass_guest_memory(guest_memory.clone());
        }
    }
}
