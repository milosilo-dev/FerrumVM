use std::fs::{self, File};

use ferrumvm::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion},
    devices::{
        cmos::Cmos,
        pci::PCI,
        serial::{Serial, SerialMode},
        timer::Pit,
        virtio::{
            devices::{blk::BlkVirtio, counter::CntVirtio, net::NetVirtio, rng::RngVirtio},
            transports::mmio::MMIOTransport,
        },
    },
    irq::map::IrqMap,
    machine_config::{
        binary::Binary,
        machine_config::{MachineConfig, MemoryRegionConfig},
    },
    vm::vm::VirtualMachine,
};

fn main() {
    print!("\n\r");
    let log_file = File::create("ferrum-firmware.log").unwrap();

    let com1 = Box::new(Serial::new(SerialMode::Terminal));
    let com2 = Box::new(Serial::new(SerialMode::LogFile(log_file)));
    let timer = Box::new(Pit::new());
    let cmos = Box::new(Cmos::new());
    let pci = Box::new(PCI::new());

    let rng = Box::new(MMIOTransport::new(Box::new(RngVirtio::new()), 1, 5));
    let cnt = Box::new(MMIOTransport::new(Box::new(CntVirtio::new()), 1, 5));
    let blk = Box::new(MMIOTransport::new(
        Box::new(BlkVirtio::new("guest/image/disk.img")),
        1,
        5,
    ));
    let net = Box::new(MMIOTransport::new(
        Box::new(NetVirtio::new()),
        2,
        6,
    ));

    let firmware = fs::read("guest/firmware/build/out.bin").unwrap();
    let firmware64 = fs::read("guest/firmware/build/main64.bin").unwrap();

    let mut machine_config = MachineConfig {
        memory_regions: vec![MemoryRegionConfig {
            mem_size: 512 * 1024 * 1024,
            mem_offset: 0x0000,
        }],
        binaries: vec![
            Binary::new(firmware, 0x7E00), // stage2 at 0x7E00
            Binary::new(firmware64, 0x100000),
            Binary::reset_vector(), // reset vector at top of first 64KB
        ],
        io_devices: vec![
            IODeviceRegion::new(0x40..=0x43, timer),
            IODeviceRegion::new(0x3f8..=0x3ff, com1),
            IODeviceRegion::new(0x2f8..=0x2ff, com2),
            IODeviceRegion::new(0x70..=0x71, cmos),
        ],
        mmio_devices: vec![
            MMIODeviceRegion::new(0x20000000..=0x20000FFF, rng),
            MMIODeviceRegion::new(0x20001000..=0x20001FFF, cnt),
            MMIODeviceRegion::new(0x20002000..=0x20002FFF, blk),
            MMIODeviceRegion::new(0x20003000..=0x20003FFF, net),
            MMIODeviceRegion::new(0xE0000000..=0xE1000000, pci),
        ],
        irq_map: IrqMap::default_map(),
        code_entry: 0xFFF0, // CPU starts executing here
    };
    machine_config.inject_acpi_tables();
    machine_config.inject_memmap();

    let mut vm = VirtualMachine::new(machine_config);

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }

    vm.dump_mem(0x0, 64); // what's at address 0? (call [rax] target when rax=0)
    vm.dump_mem(0x7080, 32);
    vm.dump_mem(0x1221820, 64);
    vm.dump_mem(0x1214CA0, 64); // return address area from stack dump
    vm.dump_mem(0x3000000, 256); // system table + handle data + AllocatePool buffer

    // shell call table: instruction at 0x1214CA3 loads RAX from [rip+0xFA43E] = [0x130F0E8]
    vm.dump_mem(0x130F0E0, 32);
    vm.dump_mem(0x121DA40, 40); // new return address
    vm.dump_mem(0x30000D8, 128 * 6); // stub protocol allocations
}
