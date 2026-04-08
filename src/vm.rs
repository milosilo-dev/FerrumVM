use kvm_bindings::{
    kvm_userspace_memory_region
};

use kvm_ioctls::{
    Kvm, 
    VcpuExit,
};

use crate::{device_maps::{
    io::{
        IODeviceMap, 
        IODeviceRegion
    }, 
    mmio::{
        MMIODeviceMap, 
        MMIODeviceRegion
    }
}, vcpu::VCPU};
use std::ptr;
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};

const MEM_SIZE: usize = 0x1000;

pub enum CrashReason {
    Hlt,
    FailedEntry,
    UnhandledExit,
    NoIODataReturned,
    IncorrectIOInputLength,
    NoMMIODataReturned,
    IncorrectMMIOReadLength,
}

pub struct VirtualMachine{
    vcpu: VCPU,
    io_map: IODeviceMap,
    mmio_map: MMIODeviceMap,
}

impl VirtualMachine{
    pub fn new(init_mem_image: Vec<u8>) -> Self{
        let kvm: Kvm = Kvm::new().unwrap();
        let vm = kvm.create_vm().unwrap();

        let raw_ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                MEM_SIZE,
                PROT_READ | PROT_WRITE,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            )
        };

        if raw_ptr == libc::MAP_FAILED {
            panic!("mmap failed");
        }

        let userspace_mem = raw_ptr as *mut u8;
        unsafe { ptr::copy_nonoverlapping(init_mem_image.as_ptr(), userspace_mem, init_mem_image.len()); }
        let memory_region = kvm_userspace_memory_region{
            slot: 0,
            flags: 0,
            guest_phys_addr: 0x1000,
            memory_size: 0x1000,
            userspace_addr: userspace_mem as u64
        };

        let _mem = unsafe { vm.set_user_memory_region(memory_region) }.unwrap();

        let io_map = IODeviceMap::new();
        let mmio_map = MMIODeviceMap::new();
        Self {
            vcpu: VCPU::new(vm),
            io_map,
            mmio_map
        }
    }

    pub fn register_io_device(&mut self, region: IODeviceRegion) {
        self.io_map.register(region);
    }

    pub fn register_mmio_device(&mut self, region: MMIODeviceRegion) {
        self.mmio_map.register(region);
    }

    pub fn run(&mut self) -> Result<(), CrashReason> {
        match self.vcpu.run() {
            VcpuExit::Hlt => {
                println!("KVM_EXIT_HLT");
                return Err(CrashReason::Hlt);
            }
            VcpuExit::IoOut(port, data) => {
                self.io_map.output(port, data);
            }
            VcpuExit::IoIn(port, data) => {
                let ret = self.io_map.input(port, data.len());
                if ret.is_none() {
                    return Err(CrashReason::NoIODataReturned);
                }
                let ret = ret.unwrap();

                if ret.len() != data.len() {
                    return Err(CrashReason::IncorrectIOInputLength);
                }
                data.copy_from_slice(&ret);
            }
            VcpuExit::MmioWrite(addr, data) => {
                self.mmio_map.write(addr, data);
            }
            VcpuExit::MmioRead(addr, data) => {
                let ret = self.mmio_map.read(addr, data.len());
                if ret.is_none() {
                    return Err(CrashReason::NoMMIODataReturned);
                }
                let ret = ret.unwrap();

                if ret.len() != data.len() {
                    return Err(CrashReason::IncorrectMMIOReadLength);
                }
                data.copy_from_slice(&ret);
            }
            VcpuExit::FailEntry(reason, ..) => {
                eprintln!(
                    "KVM_EXIT_FAIL_ENTRY: reason = {:#x}",
                    reason
                );
                return Err(CrashReason::FailedEntry);
            }
            exit_reason => {
                println!("Unhandled exit: {:?}", exit_reason);
                return Err(CrashReason::UnhandledExit);
            }
        }
        Ok(())
    }
}