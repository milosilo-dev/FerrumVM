use crate::device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion};

pub struct MemoryRegion {
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub struct Binary {
    pub data: Vec<u8>,
    pub offset: u64,
}

impl Binary {
    pub fn new(data: Vec<u8>, offset: u64) -> Self {
        Self{
            data,
            offset
        }
    }

    pub fn load_bz_image(mut data: Vec<u8>) -> Vec<Self> {
        let setup_sects = if data[0x1F1] == 0 {
            4
        } else {
            data[0x1F1] as usize
        };

        // boot_flag (0xAA55)
        data[0x1FE] = 0x55;
        data[0x1FF] = 0xAA;

        // type_of_loader
        data[0x210] = 0xFF;

        // command line location
        let cmd_ptr: u32 = 0x20000;
        data[0x228..0x22C].copy_from_slice(&cmd_ptr.to_le_bytes());

        let setup_size = (setup_sects + 1) * 512;
        let kernel_offset = setup_size;
        vec![Self {
            data: data[0..setup_size].to_vec(),
            offset: 0x10000,
        },
        Self {
            data: data[kernel_offset..].to_vec(),
            offset: 0x100000,
        },
        Self {
            data: b"console=ttyS0 earlyprintk=serial\0".to_vec(),
            offset: 0x20000,
        }]
    }
}

pub struct MachineConfig {
    pub memory_regions: Vec<MemoryRegion>,
    pub binaries: Vec<Binary>,
    pub io_devices: Vec<IODeviceRegion>,
    pub mmio_devices: Vec<MMIODeviceRegion>,

    pub code_entry: usize,
}
