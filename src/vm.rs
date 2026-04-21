use kvm_bindings::{
    KVM_IRQ_ROUTING_IRQCHIP, kvm_irq_routing, kvm_irq_routing_entry, kvm_userspace_memory_region,
};

use kvm_ioctls::{Kvm, VcpuExit, VmFd};
use vmm_sys_util::fam::FamStructWrapper;

use crate::{
    device_maps::{
        io::{IODeviceMap, IODeviceRegion},
        mmio::{MMIODeviceMap, MMIODeviceRegion},
    }, irq_handler::IRQHandler, machine_config::MachineConfig, memory_region::{GuestMemoryHandle, MemoryRegion}, vcpu::VCPU
};
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};
use std::{
    sync::{Arc, Mutex}, thread
};

pub enum CrashReason {
    Hlt,
    FailedEntry,
    UnhandledExit,
    NoIODataReturned,
    IncorrectIOInputLength,
    NoMMIODataReturned,
    IncorrectMMIOReadLength,
    Shutdown,
}

pub struct VirtualMachine {
    vcpu: VCPU,
    vm: Arc<Mutex<VmFd>>,
    io_map: Arc<Mutex<IODeviceMap>>,
    mmio_map: Arc<Mutex<MMIODeviceMap>>,
    memory_regions: GuestMemoryHandle,
}

impl VirtualMachine {
    pub fn new(mut machine_config: MachineConfig) -> Self {
        let kvm: Kvm = Kvm::new().unwrap();
        let vm = Arc::new(Mutex::new(kvm.create_vm().unwrap()));
        let _ = vm.lock().unwrap().create_irq_chip().unwrap();

        let mut routing: FamStructWrapper<kvm_irq_routing> =
            FamStructWrapper::new(machine_config.irq_map.len()).unwrap();

        let mut idx = 0;
        for irq_map in machine_config.irq_map {
            routing.as_mut_slice()[idx] = kvm_irq_routing_entry {
                gsi: irq_map.read_gsi(),
                type_: KVM_IRQ_ROUTING_IRQCHIP,
                u: kvm_bindings::kvm_irq_routing_entry__bindgen_ty_1 {
                    irqchip: kvm_bindings::kvm_irq_routing_irqchip {
                        irqchip: irq_map.read_irq_chip(),
                        pin: irq_map.read_irq_pin(),
                    },
                },
                ..Default::default()
            };
            idx += 1;
        }

        vm.lock().unwrap().set_gsi_routing(&routing).unwrap();

        let io_map = Arc::new(Mutex::new(IODeviceMap::new()));
        let mmio_map = Arc::new(Mutex::new(MMIODeviceMap::new()));
        let irq_handler = Arc::new(Mutex::new(IRQHandler::new()));
        let guest_memory = Arc::new(Mutex::new(vec![]));

        let vcpu = VCPU::new(Arc::clone(&vm), machine_config.code_entry);
        let mut this = Self {
            vcpu,
            vm: Arc::clone(&vm),
            io_map: Arc::clone(&io_map),
            mmio_map: Arc::clone(&mmio_map),
            memory_regions: guest_memory,
        };

        for mem in machine_config.memory_regions {
            this.new_mem(mem.mem_size, mem.mem_offset);
            for binary in &mut machine_config.binaries {
                if mem.mem_offset <= binary.offset as u64
                    && mem.mem_offset + mem.mem_size as u64 > binary.offset as u64
                {
                    let code_offset = binary.offset as usize - mem.mem_offset as usize;
                    let remaining = mem
                        .mem_size
                        .checked_sub(code_offset)
                        .expect("code_entry offset exceeds memory region size");

                    assert!(
                        binary.data.len() <= remaining,
                        "init_mem_image ({} bytes) overflows memory region: only {} bytes available from code entry (offset {:#x}) to end of region",
                        binary.data.len(),
                        remaining,
                        code_offset,
                    );

                    this.memory_regions.lock().unwrap().last().unwrap().write(binary.data.as_mut_slice(), code_offset);
                }
            }
        }

        for mut mmio_device in machine_config.mmio_devices {
            mmio_device.irq_handler(Arc::clone(&irq_handler));
            this.register_mmio_device(mmio_device);
        }

        for mut io_device in machine_config.io_devices {
            io_device.irq_handler(Arc::clone(&irq_handler));
            this.register_io_device(io_device);
        }

        let io_map_tick = Arc::clone(&io_map);
        let mmio_map_tick = Arc::clone(&mmio_map);
        let irq_handler_tick = Arc::clone(&irq_handler);
        let vm_tick = Arc::clone(&vm);
        thread::spawn(move || {
            loop {
                mmio_map_tick.lock().unwrap().tick();
                io_map_tick.lock().unwrap().tick();

                let mut irqs = {
                    let mut handler = irq_handler_tick.lock().unwrap();
                    handler.handle_irqs()
                };
                while let Some(irq) = irqs.pop_front() {
                    let vm_lock = vm_tick.lock().unwrap();
                    match vm_lock.set_irq_line(irq.irq_line, irq.value) {
                        Ok(_) => {}
                        Err(e) => println!("IRQ failed: {:?}", e),
                    }
                }
            }
        });

        this
    }

    fn new_mem(&mut self, mem_size: usize, mem_offset: u64) {
        let raw_ptr = unsafe {
            mmap(
                std::ptr::null_mut(),
                mem_size,
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
        self.memory_regions.lock().unwrap().push(MemoryRegion::new(userspace_mem, mem_size, mem_offset));

        let memory_region = kvm_userspace_memory_region {
            slot: self.memory_regions.lock().unwrap().len() as u32 - 1,
            flags: 0,
            guest_phys_addr: mem_offset,
            memory_size: mem_size as u64,
            userspace_addr: userspace_mem as u64,
        };

        let vm_lock = self.vm.lock().unwrap();
        let _mem = unsafe { vm_lock.set_user_memory_region(memory_region) }.unwrap();
    }

    fn register_io_device(&self, region: IODeviceRegion) -> bool {
        let io_map = self.io_map.lock();
        if io_map.is_err() {
            return false;
        }
        let mut io_map = io_map.unwrap();
        io_map.register(region);
        true
    }

    fn register_mmio_device(&self, region: MMIODeviceRegion) -> bool {
        let mmio_map = self.mmio_map.lock();
        if mmio_map.is_err() {
            return false;
        }
        let mut mmio_map = mmio_map.unwrap();
        mmio_map.register(region);
        true
    }

    pub fn run(&mut self) -> Result<(), CrashReason> {
        let exit = self.vcpu.fd.run().expect("run failed");
        match exit {
            VcpuExit::Hlt => {
                println!("KVM_EXIT_HLT");
                return Err(CrashReason::Hlt);
            }
            VcpuExit::IoOut(port, data) => {
                let mut io_map = self.io_map.lock().unwrap();
                io_map.output(port, data);
            }
            VcpuExit::IoIn(port, data) => {
                let mut io_map = self.io_map.lock().unwrap();
                let io_ret = io_map.input(port, data.len());
                if io_ret.is_none() {
                    for b in data.iter_mut() {
                        *b = 0xFF;
                    }
                    return Ok(());
                }
                let io_ret = io_ret.unwrap();

                if io_ret.len() != data.len() {
                    println!("INCORRECT_IO_INPUT_LENGTH");
                    return Err(CrashReason::IncorrectIOInputLength);
                }
                data.copy_from_slice(&io_ret);
            }
            VcpuExit::MmioWrite(addr, data) => {
                let mut mmio_map = self.mmio_map.lock().unwrap();
                mmio_map.write(addr, data);
            }
            VcpuExit::MmioRead(addr, data) => {
                let mut mmio_map = self.mmio_map.lock().unwrap();
                let io_ret = mmio_map.read(addr, data.len());
                if io_ret.is_none() {
                    for b in data.iter_mut() {
                        *b = 0;
                    }
                    return Ok(());
                }
                let io_ret = io_ret.unwrap();

                if io_ret.len() != data.len() {
                    println!("INCORRECT_MMIO_INPUT_LENGTH");
                    return Err(CrashReason::IncorrectMMIOReadLength);
                }
                data.copy_from_slice(&io_ret);
            }
            VcpuExit::FailEntry(reason, ..) => {
                eprintln!("KVM_EXIT_FAIL_ENTRY: reason = {:#x}", reason);
                return Err(CrashReason::FailedEntry);
            }
            VcpuExit::Shutdown => {
                eprintln!("KVM_SHUTDOWN");
                let regs = self.vcpu.fd.get_regs().unwrap();
                let sregs = self.vcpu.fd.get_sregs().unwrap();
                eprintln!("SHUTDOWN at RIP={:#x}", regs.rip);
                eprintln!("RAX={:#x} RBX={:#x} RCX={:#x} RDX={:#x}", regs.rax, regs.rbx, regs.rcx, regs.rdx);
                eprintln!("CR0={:#x} CR3={:#x} CR4={:#x} EFER={:#x}", sregs.cr0, sregs.cr3, sregs.cr4, sregs.efer);
                eprintln!("CS base={:#x} selector={:#x} type={:#x} l={}", sregs.cs.base, sregs.cs.selector, sregs.cs.type_, sregs.cs.l);
                return Err(CrashReason::Shutdown);
            }
            exit_reason => {
                println!("Unhandled exit: {:?}", exit_reason);
                // return Err(CrashReason::UnhandledExit);
            }
        }
        Ok(())
    }
}
