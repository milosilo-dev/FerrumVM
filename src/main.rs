use std::fs;

use skhv::{device_maps::io::IODeviceRegion, devices::serial::Serial, machine_config::{MachineConfig, MemoryRegion}, vm::VirtualMachine};

fn main() {
    let com1 = Box::new(Serial::new());
    let com2 = Box::new(Serial::new());

    let init_mem_image = fs::read("guest/firmware.bin").unwrap();
    let mut vm = VirtualMachine::new(Vec::from(init_mem_image), 
        MachineConfig{
            memory_regions: vec![MemoryRegion{mem_size: 64 * 1024 * 1024, mem_offset: 0x0000}], 
            io_devices: vec![
                IODeviceRegion::new(0x3f8..=0x3ff, com1),
                IODeviceRegion::new(0x2f8..=0x2ff, com2),
            ], 
            mmio_devices: vec![],
            code_entry: 0x0000,
        }
    );

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }
}