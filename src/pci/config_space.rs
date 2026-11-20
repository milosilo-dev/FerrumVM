use crate::{device_maps::io::IODevice, pci::bus::PCIBus};

use std::sync::{Arc, Mutex};

pub struct PciConfigSpace {
    selector: u32,
    bus: Arc<Mutex<PCIBus>>,
}

impl PciConfigSpace {
    pub fn new(bus: Arc<Mutex<PCIBus>>) -> Self {
        Self { selector: 0, bus }
    }

    fn selector_enabled(&self) -> bool {
        self.selector & (1 << 31) != 0
    }

    fn current_device(&self) -> u8 {
        ((self.selector >> 11) & 0x1F) as u8
    }

    fn current_function(&self) -> u8 {
        ((self.selector >> 8) & 0x07) as u8
    }

    fn current_dword(&self) -> u8 {
        ((self.selector >> 2) & 0x3F) as u8
    }

    fn read_config_byte(&self, dword_offset: u8) -> u8 {
        if !self.selector_enabled() {
            return 0xFF;
        }

        let Ok(bus) = self.bus.lock() else {
            return 0xFF;
        };

        bus.read_config(
            self.current_device(),
            self.current_function(),
            self.current_dword(),
            dword_offset,
        )
        .unwrap_or(0xFF)
    }

    fn write_config_byte(&self, dword_offset: u8, byte: u8) {
        if !self.selector_enabled() {
            return;
        }

        let Ok(mut bus) = self.bus.lock() else {
            return;
        };

        bus.write_config(
            self.current_device(),
            self.current_function(),
            self.current_dword(),
            dword_offset,
            byte,
        );
    }
}

impl IODevice for PciConfigSpace {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        (0..length)
            .map(|i| {
                let offset = port.wrapping_add(i as u16);
                match offset {
                    // CONFIG_ADDRESS itself is always readable.
                    0x00..=0x03 => (self.selector >> (8 * offset as u32)) as u8,
                    // CONFIG_DATA, one byte per port.
                    0x04..=0x07 => self.read_config_byte((offset - 0x04) as u8),
                    // Unused bytes of the port range.
                    _ => 0xFF,
                }
            })
            .collect()
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        for (i, byte) in data.iter().enumerate() {
            let offset = port.wrapping_add(i as u16);
            match offset {
                0x00..=0x03 => {
                    let shift = 8 * offset as u32;
                    self.selector =
                        (self.selector & !(0xFFu32 << shift)) | ((*byte as u32) << shift);
                }
                0x04..=0x07 => self.write_config_byte((offset - 0x04) as u8, *byte),
                _ => {}
            }
        }
    }
}
