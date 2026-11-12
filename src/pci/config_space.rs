use crate::device_maps::io::IODevice;

/// pci config io device, map at 0xCF8 - 0xCFC, only the first and last port are used howverer
pub struct PciConfigSpace {
    selector_enabled: bool,
    current_bus: u8,
    current_device: u8,
    current_function: u8,
    current_dword: u8,
}

impl IODevice for PciConfigSpace {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        vec![]
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        match port {
            0x00 => {
                if data.len() < 4 {
                    return;
                }
                let data_full: u32 = u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
                self.current_dword = ((data_full >> 2) & 0x3F) as u8;
                self.current_function = ((data_full >> 8) & 0b111) as u8;
                self.current_device = ((data_full >> 11) & 0x1F) as u8;
                self.current_bus = ((data_full >> 16) & 0xFF) as u8;
                self.selector_enabled = ((data_full >> 31) & 0x1) != 0;
            }
            0x04..=0x07 => {
                // write a dword from the selected functions config space
            }
            _ => {
                // Nothing
            }
        }
    }
}
