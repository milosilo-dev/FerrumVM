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
            // Conventional low RAM (IVT, BDA, free conventional)
            MemMap { start: 0x00000,    end: 0x9EFFF,    mem_type: MemType::ConventionalMemory as u32 },
            // EBDA
            MemMap { start: 0x9F000,    end: 0x9FFFF,    mem_type: MemType::ACPIReclaimMemory as u32 },
            // VGA framebuffer + option ROMs — KVM does NOT back these with RAM
            MemMap { start: 0xA0000,    end: 0xDFFFF,    mem_type: MemType::Reserved as u32 },
            // BIOS ROM shadow
            MemMap { start: 0xE0000,    end: 0xFFFFF,    mem_type: MemType::Reserved as u32 },
            // Your firmware image
            MemMap { start: 0x100000,   end: 0x1FFFFF,   mem_type: MemType::BootServicesCode as u32 },
            // Free RAM
            MemMap { start: 0x200000,   end: 0x11FFFFF,  mem_type: MemType::ConventionalMemory as u32 },
            // Whatever lives here (Limine loaded? Reserved?)
            MemMap { start: 0x1200000,  end: 0x14FFFFF,  mem_type: MemType::Reserved as u32 },
            // More free RAM
            MemMap { start: 0x1500000,  end: 0x1FEFFFFF, mem_type: MemType::ConventionalMemory as u32 },
            // MMIO
            MemMap { start: 0x20000000, end: 0x2000FFFF, mem_type: MemType::MMIO as u32 },
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