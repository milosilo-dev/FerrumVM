use crate::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion}, irq::map::IrqMap, machine_config::{
        acpi::{dsdt::load_dsdt, fadt::build_fadt, madt::build_madt, rsdp::build_rsdp, xsdt::build_xsdt}, binary::Binary, mem_map::{MemMap, MemMapHeader, MemType},
    },
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
    pub total_vcpus: u8,
}

impl MachineConfig {
    pub fn inject_memmap(&mut self) {
        let ram_end =
            self.memory_regions[0].mem_size as u64 - self.memory_regions[0].mem_offset;

        // The in-kernel KVM irqchip owns these ranges (no user RAM slot backs them), so
        // they must be excluded from usable RAM. Reserve the whole span from the IOAPIC
        // through the LAPIC as ONE hole so the memory map stays small (Limine's internal
        // memory-map array overflows - "Memory map exhausted." - if given too many /
        // over-fragmented entries).
        const APIC_RESERVED_START: u64 = 0xFEC0_0000;
        const APIC_RESERVED_END: u64 = 0xFEE0_1000;

        // Keep the map minimal but non-overlapping: low memory below the first RAM region
        // is described as a few fixed regions, then a single conventional region spans all
        // the way up to the end of guest RAM, with the APIC span carved out as reserved.
        let mut mem_map: Vec<MemMap> = vec![
            // Real mode IVT + BDA - keep reserved
            MemMap {
                start: 0x00000,
                end: 0x00500,
                mem_type: MemType::Reserved as u32,
            },
            // Free conventional low memory (trampoline)
            MemMap {
                start: 0x00500,
                end: 0x9F000,
                mem_type: MemType::ConventionalMemory as u32,
            },
            // EBDA
            MemMap {
                start: 0x9F000,
                end: 0xA0000,
                mem_type: MemType::ACPIReclaimMemory as u32,
            },
            // VGA framebuffer + option ROMs + BIOS ROM shadow — KVM does NOT back these
            MemMap {
                start: 0xA0000,
                end: 0x100000,
                mem_type: MemType::Reserved as u32,
            },
            // Your firmware image
            MemMap {
                start: 0x100000,
                end: 0x200000,
                mem_type: MemType::BootServicesCode as u32,
            },
            // Conventional RAM from the firmware image up to the firmware heap
            MemMap {
                start: 0x200000,
                end: 0x3000000,
                mem_type: MemType::ConventionalMemory as u32,
            },
            // Firmware heap (0x3000000-0x4000000): holds the EFI runtime memory map
            // buffer and runtime tables handed to the OS. Reported as runtime services
            // data so Linux reserves and keeps it mapped (and Limine doesn't reclaim it
            // as usable RAM) instead of faulting on the map during efi_set_virtual_address_map.
            MemMap {
                start: 0x3000000,
                end: 0x4000000,
                mem_type: MemType::RuntimeServicesData as u32,
            },
            // Conventional RAM from the firmware heap up to the APIC span
            MemMap {
                start: 0x4000000,
                end: APIC_RESERVED_START,
                mem_type: MemType::ConventionalMemory as u32,
            },
            // IOAPIC + LAPIC (in-kernel irqchip) - reserved
            MemMap {
                start: APIC_RESERVED_START,
                end: APIC_RESERVED_END,
                mem_type: MemType::Reserved as u32,
            },
            // Conventional RAM from the APIC span up to the end of guest RAM
            MemMap {
                start: APIC_RESERVED_END,
                end: ram_end,
                mem_type: MemType::ConventionalMemory as u32,
            },
            // MMIO
            MemMap {
                start: 0x400000000,
                end: 0x400010000,
                mem_type: MemType::MMIO as u32,
            },
        ];

        // UEFI/Limine expect the memory map sorted by ascending PhysicalStart.
        mem_map.sort_by_key(|m| m.start);

        let mut memmap_bytes = MemMapHeader {
            mgk_num: 0xFE02FE02,
            length: mem_map.len() as u32,
        }
        .as_bytes();

        for entry in mem_map {
            memmap_bytes.extend(entry.as_bytes());
        }

        self.binaries.push(Binary {
            data: memmap_bytes,
            offset: 0x7000,
        });
    }

    pub fn inject_acpi_tables(&mut self) {
        let dsdt_bin = load_dsdt();
        let fadt_bin = build_fadt(dsdt_bin.offset);
        let madt_bin = build_madt(self.total_vcpus);
        let xsdt_bin = build_xsdt(&[fadt_bin.offset, madt_bin.offset]);
        let rsdp_bin = build_rsdp(xsdt_bin.offset);

        self.binaries.push(dsdt_bin);
        self.binaries.push(fadt_bin);
        self.binaries.push(madt_bin);
        self.binaries.push(rsdp_bin);
        self.binaries.push(xsdt_bin);
    }
}
