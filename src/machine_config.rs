use crate::device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion};

pub struct MemoryRegion {
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub struct MachineConfig {
    pub memory_regions: Vec<MemoryRegion>,
    pub io_devices: Vec<IODeviceRegion>,
    pub mmio_devices: Vec<MMIODeviceRegion>,

    pub code_entry: usize,
}