use std::{
    fs::{self, File},
    os::fd::AsRawFd,
    path::PathBuf,
};

use crossterm::terminal::disable_raw_mode;

use ferrumvm::{
    device_maps::{io::IODeviceRegion, mmio::MMIODeviceRegion},
    devices::{
        cmos::Cmos,
        serial::{Serial, SerialMode},
        timer::Pit,
        virtio::{
            devices::{
                blk::BlkVirtio, counter::CntVirtio, fs::FsVirtio, net::NetVirtio, rng::RngVirtio,
            },
            transports::mmio::MMIOTransport,
        },
    },
    irq::map::IrqMap,
    machine_config::{
        binary::Binary,
        machine_config::{MachineConfig, MemoryRegionConfig},
    },
    platform::{networking::tap::TAPDevice, shared_folder::SharedFolder},
    vm::vm::VirtualMachine,
};

fn main() {
    {
        let host_log = File::create("ferrum-host.log").expect("failed to create ferrum-host.log");
        unsafe {
            libc::dup2(host_log.as_raw_fd(), libc::STDERR_FILENO);
        }
    }

    let tap_device = match TAPDevice::new() {
        Ok(tap_device) => tap_device,
        Err(err) => {
            eprint!("{}\r\n", err);
            panic!("{}", err);
        }
    };

    print!("\n\r");
    let firmware_log_file = File::create("ferrum-firmware.log").unwrap();
    let kernel_log_file = File::create("ferrum-kernel.log").unwrap();

    let com1 = Box::new(Serial::new(SerialMode::Terminal));
    let com2 = Box::new(Serial::new(SerialMode::LogFile(firmware_log_file)));
    let com3 = Box::new(Serial::new(SerialMode::LogFile(kernel_log_file)));

    let timer = Box::new(Pit::new());
    let cmos = Box::new(Cmos::new());

    let rng = Box::new(MMIOTransport::new(Box::new(RngVirtio::new()), 1, 0));
    let cnt = Box::new(MMIOTransport::new(Box::new(CntVirtio::new()), 1, 0));
    let blk = Box::new(MMIOTransport::new(
        Box::new(BlkVirtio::new("guest/image/disk.img")),
        1,
        5,
    ));
    let net = Box::new(MMIOTransport::new(
        Box::new(NetVirtio::new(tap_device)),
        2,
        6,
    ));
    let fuse = Box::new(MMIOTransport::new(
        Box::new(FsVirtio::new(
            "Self",
            SharedFolder::new(PathBuf::from("/home/miles/Data/FerrumVM/")),
        )),
        2,
        7,
    ));

    let firmware = fs::read("guest/firmware/build/out.bin").unwrap();
    let firmware64 = fs::read("guest/firmware/build/main64.bin").unwrap();

    let mut machine_config = MachineConfig {
        memory_regions: vec![MemoryRegionConfig {
            mem_size: 1024 * 1024 * 1024, // 1Gb
            mem_offset: 0x0000,
        }],
        binaries: vec![
            Binary::new(firmware, 0x7E00),     // stage2 at 0x7E00
            Binary::new(firmware64, 0x100000), // long mode at 0x100000
            Binary::reset_vector(),            // reset vector at top of first 64KB
        ],
        io_devices: vec![
            IODeviceRegion::new(0x40..=0x43, timer),
            IODeviceRegion::new(0x3f8..=0x3ff, com1),
            IODeviceRegion::new(0x2f8..=0x2ff, com2),
            IODeviceRegion::new(0x3E8..=0x3EF, com3),
            IODeviceRegion::new(0x70..=0x71, cmos),
        ],
        mmio_devices: vec![
            MMIODeviceRegion::new(0xFFF00000..=0xFFF00FFF, rng),
            MMIODeviceRegion::new(0xFFF01000..=0xFFF01FFF, cnt),
            MMIODeviceRegion::new(0xFFF02000..=0xFFF02FFF, blk),
            MMIODeviceRegion::new(0xFFF03000..=0xFFF03FFF, net),
            MMIODeviceRegion::new(0xFFF04000..=0xFFF04FFF, fuse),
        ],
        irq_map: IrqMap::default_map(),
        code_entry: 0xFFF0, // CPU starts executing here
    };
    machine_config.inject_memmap();

    let mut vm = VirtualMachine::new(machine_config);

    loop {
        let ret = vm.run();
        if ret.is_err() {
            break;
        }
    }

    disable_raw_mode().unwrap();
    print!("VM Crash!\n");
}
