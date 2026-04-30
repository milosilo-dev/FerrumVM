use std::ops::{Range};

use crate::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion}, irq::map::IrqMap, machine_config::{binary::Binary, mem_map::{MemMap, MemMapHeader, MemMapType}},
};

pub struct MemoryRegionConfig {
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub struct MachineConfig {
    pub memory_regions: Vec<MemoryRegionConfig>,
    pub binaries: Vec<Binary>,
    pub io_devices: Vec<IODeviceRegion>,
    pub mmio_devices: Vec<MMIODeviceRegion>,
    pub irq_map: Vec<IrqMap>,

    pub code_entry: usize,
}

fn range_len(start: u64, end: u64) -> u128 {
    (end as u128) - (start as u128) + 1
}

impl MachineConfig {
    pub fn inject_memmap(&mut self, usable_range_option: Option<Range<u64>>) {
        // Build a mem map struct from binaries and mmio
        let mut mem_map: Vec<MemMap> = vec![];

        let mut last_binary_addr: u64 = 0;
        for binary in &mut self.binaries{
            last_binary_addr = last_binary_addr.max(binary.offset + binary.data.len() as u64);
            mem_map.push(MemMap{
                base: binary.offset,
                length: binary.data.len() as u64,
                mem_type: MemMapType::Reserved as u32,
            });
        }

        let mut first_mmio_addr: u64 = u64::MAX;
        for mmio in &mut self.mmio_devices{
            let range = mmio.get_range();
            first_mmio_addr = first_mmio_addr.min(*range.start());
            mem_map.push(MemMap {
                base: *range.start(),
                length: range_len(*range.start(), *range.end()) as u64, 
                mem_type: MemMapType::Reserved as u32
            });
        }

        let usable_range = if usable_range_option.is_some() {
            usable_range_option.unwrap()
        } else {
            last_binary_addr..first_mmio_addr
        };

        mem_map.push(MemMap { base: usable_range.start, length: usable_range.end - usable_range.start, mem_type: MemMapType::Usable as u32 });
        // Convert it to bytes
        let mem_map_header = MemMapHeader {
            mgk_num: 0xFE02FE02,
            length: (mem_map.len() + 1) as u32,
        };

        let mut memmap_bytes: Vec<u8> = mem_map_header.as_bytes();
        for mem_map_entry in &mut mem_map{
            let memmap_entry_bytes = mem_map_entry.as_bytes();
            memmap_bytes.extend_from_slice(&memmap_entry_bytes);
        }

        let memmap2_entry = MemMap{
            base: 0x7000,
            length: (mem_map.len() as u64 + 1) * 20 + 8,
            mem_type: 2,
        };
        let memmap_entry_bytes = memmap2_entry.as_bytes();
        memmap_bytes.extend_from_slice(&memmap_entry_bytes);

        // inject it as new binary
        self.binaries.push(Binary { data: memmap_bytes, offset: 0x7000 });
    }
}