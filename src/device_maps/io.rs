use std::{
    ops::RangeInclusive,
    sync::{Arc, Mutex},
};

use crate::irq::handler::IRQHandler;

pub trait IODevice: Send {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8>;
    fn output(&mut self, port: u16, data: &[u8]);
    fn irq_handler(&mut self, _irq_handler: Arc<Mutex<IRQHandler>>) {}
    fn tick(&mut self) {}
}

pub struct IODeviceRegion {
    io_device: Box<dyn IODevice>,
    port_range: RangeInclusive<u16>,
}

impl IODeviceRegion {
    pub fn new(range: RangeInclusive<u16>, device: Box<dyn IODevice>) -> Self {
        Self {
            io_device: device,
            port_range: range,
        }
    }

    pub fn contains(&self, port: u16) -> bool {
        self.port_range.contains(&port)
    }

    pub fn input(&mut self, port: u16, length: usize) -> Option<Vec<u8>> {
        if !self.port_range.contains(&port) {
            return None;
        }
        Some(
            self.io_device
                .input(port - *self.port_range.start(), length),
        )
    }

    pub fn output(&mut self, port: u16, data: &[u8]) -> Option<()> {
        if !self.port_range.contains(&port) {
            return None;
        }
        self.io_device.output(port - *self.port_range.start(), data);
        Some(())
    }

    pub fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.io_device.irq_handler(irq_handler);
    }

    pub fn tick(&mut self) {
        self.io_device.tick();
    }
}

pub struct IODeviceMap {
    devices: Vec<IODeviceRegion>,
}

impl IODeviceMap {
    pub fn new() -> Self {
        Self { devices: vec![] }
    }

    pub fn register(&mut self, region: IODeviceRegion) {
        self.devices.push(region);
    }

    pub fn input(&mut self, port: u16, length: usize) -> Option<Vec<u8>> {
        for device in &mut self.devices {
            if device.contains(port) {
                return device.input(port, length);
            }
        }
        None
    }

    pub fn output(&mut self, port: u16, data: &[u8]) -> Option<()> {
        for device in &mut self.devices {
            if device.contains(port) {
                device.output(port, data);
                return Some(());
            }
        }
        None
    }

    pub fn tick(&mut self) {
        for dev in &mut self.devices {
            dev.tick();
        }
    }
}
