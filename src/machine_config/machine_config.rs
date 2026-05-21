use crate::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion}, irq::map::IrqMap, machine_config::{acpi::{dsdt::load_dsdt, fadt::build_fadt, rsdp::build_rsdp, xsdt::build_xsdt}, binary::Binary, mem_map::{MemMap, MemMapHeader, MemType}},
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

impl MachineConfig {
    pub fn inject_memmap(&mut self) {
        let mem_map: Vec<MemMap> = vec![
            MemMap{start: 0x0000000, end: 0x009F000,  mem_type: MemType::Unusable as u32},
            MemMap{start: 0x009F000, end: 0x00A0000,  mem_type: MemType::ACPIReclaimMemory as u32},
            MemMap{start: 0x00E0800, end: 0x00F0000,  mem_type: MemType::ConventionalMemory as u32},
            MemMap{start: 0x00F0000, end: 0x0100000,  mem_type: MemType::RuntimeServicesCode as u32},
            MemMap{start: 0x0100000, end: 0x0200000,  mem_type: MemType::BootServicesCode as u32},
            MemMap{start: 0x0200000, end: 0x1200000,  mem_type: MemType::ConventionalMemory as u32},
            MemMap{start: 0x1200000, end: 0x1500000,  mem_type: MemType::LoaderCode as u32},
            MemMap{start: 0x1500000, end: 0x20000000, mem_type: MemType::ConventionalMemory as u32},
            MemMap{start: 0x20000000, end: 0x20010000, mem_type: MemType::MMIO as u32},
        ];

        let mut memmap_bytes = MemMapHeader{
            mgk_num: 0xFE02FE02,
            length: mem_map.len() as u32,
        }.as_bytes();

        for entry in mem_map {
            memmap_bytes.extend(entry.as_bytes());
        }

        self.binaries.push(Binary { data: memmap_bytes, offset: 0x7000 });
    }

    pub fn inject_acpi_tables(&mut self) {
        let dsdt_bin = load_dsdt();
        let fadt_bin = build_fadt(dsdt_bin.offset);
        let xsdt_bin = build_xsdt(&[fadt_bin.offset]);
        let rsdp_bin = build_rsdp(xsdt_bin.offset);

        self.binaries.push(dsdt_bin);
        self.binaries.push(fadt_bin);
        self.binaries.push(rsdp_bin);
        self.binaries.push(xsdt_bin);
    }
}