use crate::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion},
    irq_map::IrqMap,
};

pub struct MemoryRegionConfig {
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub struct Binary {
    pub data: Vec<u8>,
    pub offset: u64,
}

impl Binary {
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        Self { data, offset }
    }

    fn build_boot_params(bzimage: &[u8]) -> Vec<u8> {
        let mut bp = vec![0u8; 4096]; // 4KB zeroed

        // Copy setup_header (starts at 0x1F1 in bzImage)
        let setup_header_start = 0x1F1;
        let setup_header_size = 0x100; // enough to cover needed fields

        bp[0x1F1..0x1F1 + setup_header_size]
            .copy_from_slice(&bzimage[setup_header_start..setup_header_start + setup_header_size]);

        // Set command line pointer
        let cmdline_addr: u32 = 0x21000;

        bp[0x228..0x22C].copy_from_slice(&cmdline_addr.to_le_bytes());

        // cmdline size
        let cmdline = b"console=ttyS0 earlyprintk=serial\0";
        let size = cmdline.len() as u32;
        bp[0x238..0x23C].copy_from_slice(&size.to_le_bytes());

        bp
    }

    pub fn load_bz_image(data: Vec<u8>) -> Vec<Self> {
        let setup_sects = if data[0x1F1] == 0 {
            4
        } else {
            data[0x1F1] as usize
        };

        let setup_size = (setup_sects + 1) * 512;
        let kernel_offset = setup_size;

        let boot_params = Self::build_boot_params(&data);
        vec![
            Self {
                data: data[0..setup_size].to_vec(),
                offset: 0x10000,
            },
            Self {
                data: data[kernel_offset..].to_vec(),
                offset: 0x100000,
            },
            Self {
                data: boot_params,
                offset: 0x20000,
            },
            Self {
                data: b"console=ttyS0 earlyprintk=serial\0".to_vec(),
                offset: 0x21000,
            },
        ]
    }
}

pub struct MachineConfig {
    pub memory_regions: Vec<MemoryRegionConfig>,
    pub binaries: Vec<Binary>,
    pub io_devices: Vec<IODeviceRegion>,
    pub mmio_devices: Vec<MMIODeviceRegion>,
    pub irq_map: Vec<IrqMap>,

    pub code_entry: usize,
}
