use std::fs;

use skhv::{
    device_maps::io::IODeviceRegion,
    devices::{cmos::Cmos, serial::Serial, timer::Pit},
    irq_map::IrqMap,
    machine_config::{Binary, MachineConfig, MemoryRegionConfig},
    vm::VirtualMachine,
};

fn main() {
    let com1 = Box::new(Serial::new());
    let com2 = Box::new(Serial::new());

    let timer = Box::new(Pit::new());
    let cmos = Box::new(Cmos::new());

    let init_mem_image = fs::read("guest/clock.bin").unwrap();
    let mut vm = VirtualMachine::new(MachineConfig {
        memory_regions: vec![MemoryRegionConfig {
            mem_size: 64 * 1024 * 1024,
            mem_offset: 0x0000,
        }],
        binaries: vec![Binary::new(init_mem_image, 0x1000)],
        io_devices: vec![
            IODeviceRegion::new(0x40..=0x43, timer),
            IODeviceRegion::new(0x3f8..=0x3ff, com1),
            IODeviceRegion::new(0x2f8..=0x2ff, com2),
            IODeviceRegion::new(0x70..=0x71, cmos),
        ],
        mmio_devices: vec![],
        irq_map: IrqMap::default_map(),
        code_entry: 0x1000,
    });

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }
}
