use kvm_bindings::{
    KVM_IRQ_ROUTING_IRQCHIP, kvm_irq_routing, kvm_irq_routing_entry, kvm_userspace_memory_region,
};

use kvm_ioctls::{Kvm, VcpuExit, VmFd};
use vmm_sys_util::fam::FamStructWrapper;

use crate::{
    device_maps::{
        io::{IODeviceMap, IODeviceRegion},
        mmio::{MMIODeviceMap, MMIODeviceRegion},
    },
    irq_handler::IRQHandler,
    machine_config::MachineConfig,
    vcpu::VCPU,
};
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};
use std::{
    ptr,
    sync::{Arc, Mutex},
    thread::{self, sleep},
    time::Duration,
};

pub enum CrashReason {
    Hlt,
    FailedEntry,
    UnhandledExit,
    NoIODataReturned,
    IncorrectIOInputLength,
    NoMMIODataReturned,
    IncorrectMMIOReadLength,
}

pub struct VirtualMachine {
    vcpu: VCPU,
    vm: Arc<Mutex<VmFd>>,
    io_map: Arc<Mutex<IODeviceMap>>,
    mmio_map: Arc<Mutex<MMIODeviceMap>>,
    memory_regions: Vec<*mut u8>,
}

impl VirtualMachine {
    pub fn new(machine_config: MachineConfig) -> Self {
        let kvm: Kvm = Kvm::new().unwrap();
        let vm = Arc::new(Mutex::new(kvm.create_vm().unwrap()));
        let _ = vm.lock().unwrap().create_irq_chip().unwrap();

        let mut routing: FamStructWrapper<kvm_irq_routing> = FamStructWrapper::new(1).unwrap();

        routing.as_mut_slice()[0] = kvm_irq_routing_entry {
            gsi: 1,
            type_: KVM_IRQ_ROUTING_IRQCHIP,
            u: kvm_bindings::kvm_irq_routing_entry__bindgen_ty_1 {
                irqchip: kvm_bindings::kvm_irq_routing_irqchip { irqchip: 0, pin: 1 },
            },
            ..Default::default()
        };

        vm.lock().unwrap().set_gsi_routing(&routing).unwrap();

        let io_map = Arc::new(Mutex::new(IODeviceMap::new()));
        let mmio_map = Arc::new(Mutex::new(MMIODeviceMap::new()));
        let irq_handler = Arc::new(Mutex::new(IRQHandler::new()));

        let vcpu = VCPU::new(Arc::clone(&vm), machine_config.code_entry);
        let mut this = Self {
            vcpu,
            vm: Arc::clone(&vm),
            io_map: Arc::clone(&io_map),
            mmio_map: Arc::clone(&mmio_map),
            memory_regions: vec![],
        };

        for mem in machine_config.memory_regions {
            this.new_mem(mem.mem_size, mem.mem_offset);
            for binary in &machine_config.binaries {
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

                    unsafe {
                        ptr::copy_nonoverlapping(
                            binary.data.as_ptr(),
                            this.memory_regions
                                .last()
                                .expect("Can't find memory region")
                                .add(code_offset),
                            binary.data.len(),
                        );
                    }
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
            //let evt = EventFd::new(0).unwrap();
            {
                //let vm_lock = vm_tick.lock().unwrap();
                //vm_lock.register_irqfd(&evt, 0).unwrap();
            }

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

                    //evt.write(1).unwrap();
                }

                sleep(Duration::from_millis(1));
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
        self.memory_regions.push(userspace_mem);

        let memory_region = kvm_userspace_memory_region {
            slot: self.memory_regions.len() as u32 - 1,
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
            exit_reason => {
                println!("Unhandled exit: {:?}", exit_reason);
                // return Err(CrashReason::UnhandledExit);
            }
        }
        Ok(())
    }
}
