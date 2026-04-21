use std::fs;

use skhv::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion},
    devices::{cmos::Cmos, serial::Serial, timer::Pit, virtio::{devices::rng::RngVirtio, transports::mmio::MMIOTransport}},
    irq_map::IrqMap,
    machine_config::{Binary, MachineConfig, MemoryRegionConfig},
    vm::VirtualMachine,
};

fn main() {
    let com1 = Box::new(Serial::new());
    let com2 = Box::new(Serial::new());

    let timer = Box::new(Pit::new());
    let cmos = Box::new(Cmos::new());

    let rng = Box::new(MMIOTransport::new(Box::new(RngVirtio::new()), 1));

    let init_mem_image = fs::read("guest/linuxBzImage").unwrap();
    let mut vm = VirtualMachine::new(MachineConfig {
        memory_regions: vec![MemoryRegionConfig {
            mem_size: 64 * 1024 * 1024,
            mem_offset: 0x0000,
        }],
        binaries: Binary::load_bz_image(init_mem_image),
        io_devices: vec![
            IODeviceRegion::new(0x40..=0x43, timer),
            IODeviceRegion::new(0x3f8..=0x3ff, com1),
            IODeviceRegion::new(0x2f8..=0x2ff, com2),
            IODeviceRegion::new(0x70..=0x71, cmos),
        ],
        mmio_devices: vec![
            MMIODeviceRegion::new(0x10001000..=0x10001FFF, rng),
        ],
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
