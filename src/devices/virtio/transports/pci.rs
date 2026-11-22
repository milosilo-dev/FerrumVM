use crate::{
    devices::virtio::virtio::VirtioDevice,
    pci::{config_instance::ConfigInstance, device::PCIDevice},
};

use std::ops::RangeInclusive;

const BAR0_SIZE: u64 = 0x4000; // 16 KiB: 4 x 4 KiB sub-regions
const BAR0_FLAGS: u64 = 0x4; // memory space + 64-bit type

pub struct PCITransport {
    config: ConfigInstance,
    device: Box<dyn VirtioDevice + Send>,
    bar0: u64,      // raw register contents
    bar0_base: u64, // decoded base address
}

impl PCITransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize, irq_sel: u32) -> Self {
        let mut config = ConfigInstance::new();
        config.add_config_region(vec![0xFA, 0x1A], 0x0, false); // Vendor
        config.add_config_region(
            (0x1040 + device.virtio_type()).to_le_bytes().to_vec(),
            0x02,
            false,
        ); // Device type
        config.add_config_region(vec![0x0, 0x0], 0x04, true); // Command
        config.add_config_region(vec![0x0, 0x0], 0x06, true); // Status
        config.add_config_region(vec![0x01], 0x08, false); // Revision
        config.add_config_region(vec![0x0], 0x09, false); // Prog IF
        config.add_config_region(
            vec![if device.virtio_type() == 0x01 {
                0x0
            } else {
                0x80
            }],
            0x0A,
            false,
        ); // Subclass
        config.add_config_region(
            vec![match device.virtio_type() {
                0x01 => 0x02,
                0x02 => 0x01,
                0x03 => 0x07,
                _ => 0x0,
            }],
            0x0B,
            false,
        ); // Class
        config.add_config_region(vec![0x0], 0x0E, false); // Header type
        config.add_config_region(vec![0x0; 8], 0x10, true); // BAR 0
        config.add_config_region(vec![0x0; 16], 0x18, false); // BAR 2-5
        config.add_config_region(0x1AF4u16.to_le_bytes().to_vec(), 0x2C, false); // Subsystem vendor
        config.add_config_region(0x1100u16.to_le_bytes().to_vec(), 0x2E, false); // Subsystem ID
        config.add_config_region(vec![0x40], 0x34, false); // Cap pointer
        config.add_config_region(vec![0x01], 0x3D, false); // Interrupt pin

        Self {
            config,
            device,
            bar0: 0,
            bar0_base: 0,
        }
    }

    fn bar_read(&self, off: u16) -> u8 {
        self.bar0.to_le_bytes()[off as usize]
    }

    fn bar_write(&mut self, off: u16, value: u8) {
        let mut bytes = self.bar0.to_le_bytes();
        bytes[off as usize] = value;
        self.bar0 = u64::from_le_bytes(bytes);

        if self.bar0 == u64::MAX {
            // guest is probing size: report size mask, not the written value
            self.bar0 = (!(BAR0_SIZE - 1)) | BAR0_FLAGS;
        } else {
            // guest assigned a real base; clear the 4 flag bits
            self.bar0_base = self.bar0 & !0xF;
        }
    }
}

impl PCIDevice for PCITransport {
    fn read_config(&self, function: &u8, dword: &u8, dword_offset: &u8) -> Option<u8> {
        if *function != 0 {
            return Some(0xFF);
        }
        let off = (*dword as u16) * 4 + *dword_offset as u16;
        match off {
            0x10..=0x17 => Some(self.bar_read(off - 0x10)),
            _ => self.config.read(off),
        }
    }

    fn write_config(&mut self, function: &u8, dword: &u8, dword_offset: &u8, data: u8) {
        if *function != 0 {
            return;
        }
        let off = (*dword as u16) * 4 + *dword_offset as u16;

        match off {
            0x10..=0x17 => self.bar_write(off - 0x10, data),
            _ => self.config.write(off, data),
        }
    }

    fn mmio_read(&self, addr: u64) -> Option<u8> {
        Some(0)
    }

    fn mmio_write(&mut self, addr: u64, value: u8) {}

    fn get_mmio_range(&self) -> RangeInclusive<u64> {
        self.bar0_base..=self.bar0_base + BAR0_SIZE - 1
    }
}
