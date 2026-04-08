use std::fs;

use skhv::{device_maps::io::IODeviceRegion, devices::serial::Serial, vm::VirtualMachine};

fn main() {
    let init_mem_image = fs::read("guest/firmware.bin").unwrap();
    let mut vm = VirtualMachine::new(Vec::from(init_mem_image));

    let serial_device = Box::new(Serial{});
    vm.register_io_device(IODeviceRegion::new(0x3f8..=0x3f8, serial_device));

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }
}