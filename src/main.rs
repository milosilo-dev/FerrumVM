use std::fs;

use ferrumvm::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion},
    devices::{cmos::Cmos, serial::Serial, timer::Pit, virtio::{devices::{blk::BlkVirtio, counter::CntVirtio, rng::RngVirtio}, transports::mmio::MMIOTransport}},
    irq::map::IrqMap,
    machine_config::{binary::Binary, machine_config::{MachineConfig, MemoryRegionConfig}},
    vm::vm::VirtualMachine,
};

fn main() {
    let com1 = Box::new(Serial::new());
    let com2 = Box::new(Serial::new());
    let timer = Box::new(Pit::new());
    let cmos = Box::new(Cmos::new());
    let rng = Box::new(MMIOTransport::new(Box::new(RngVirtio::new()), 1));
    let cnt = Box::new(MMIOTransport::new(Box::new(CntVirtio::new()), 1));
    let blk = Box::new(MMIOTransport::new(Box::new(BlkVirtio::new("guest/disk.bin")), 1));

    let firmware = fs::read("guest/firmware/build/out.bin").unwrap();
    let firmware64 = fs::read("guest/firmware/build/main64.bin").unwrap();

    let mut machine_config = MachineConfig {
        memory_regions: vec![MemoryRegionConfig {
            mem_size: 64 * 1024 * 1024,
            mem_offset: 0x0000,
        }],
        binaries: vec![
            Binary::new(firmware,       0x7E00),  // stage2 at 0x7E00
            Binary::new(firmware64,     0x100000),
            Binary::reset_vector(),  // reset vector at top of first 64KB
        ],
        io_devices: vec![
            IODeviceRegion::new(0x40..=0x43, timer),
            IODeviceRegion::new(0x3f8..=0x3ff, com1),
            IODeviceRegion::new(0x2f8..=0x2ff, com2),
            IODeviceRegion::new(0x70..=0x71, cmos),
        ],
        mmio_devices: vec![
            MMIODeviceRegion::new(0x10001000..=0x10001FFF, rng),
            MMIODeviceRegion::new(0x10002000..=0x10002FFF, cnt),
            MMIODeviceRegion::new(0x10003000..=0x10003FFF, blk),
        ],
        irq_map: IrqMap::default_map(),
        code_entry: 0xFFF0,  // CPU starts executing here
    };
    machine_config.inject_memmap(None);

    let mut vm = VirtualMachine::new(machine_config);

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }
}