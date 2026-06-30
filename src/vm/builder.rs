use std::sync::{Arc, Mutex};

use kvm_bindings::{
    KVM_IRQ_ROUTING_IRQCHIP, kvm_irq_routing, kvm_irq_routing_entry, kvm_userspace_memory_region,
};
use kvm_ioctls::Kvm;
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};
use vmm_sys_util::fam::FamStructWrapper;

use crate::{
    device_maps::{
        io::{IODeviceMap, IODeviceRegion},
        mmio::{MMIODeviceMap, MMIODeviceRegion},
    },
    irq::handler::IRQHandler,
    machine_config::machine_config::MachineConfig,
    memory_region::MemoryRegion,
    vcpu::VCPU,
    vm::{tick::TickContext, vm::VirtualMachine},
};

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

        let mut cpuid = kvm
            .get_supported_cpuid(kvm_bindings::KVM_MAX_CPUID_ENTRIES)
            .unwrap();
        for entry in cpuid.as_mut_slice() {
            match entry.function {
                0x80000000 => {
                    if entry.eax < 0x80000001 {
                        entry.eax = 0x80000001;
                    }
                }

                0x80000001 => {
                    entry.edx |= 1 << 29; // Long mode
                    entry.edx |= 1 << 20; // NX
                }

                _ => {}
            }
        }

        let vcpu = VCPU::new(Arc::clone(&vm), machine_config.code_entry, &mut cpuid);

        // Unmask LAPIC LVT0 (ExtINT) so PIC interrupts from set_irq_line reach the CPU.
        // KVM resets LVT0 to 0x10700 (ExtINT + masked). Without unmasking, every device
        // interrupt delivered through the PIC (virtio-blk, virtio-net, serial, keyboard)
        // is silently dropped.
        {
            use std::io::Write;
            const APIC_LVT0: usize = 0x350;
            let mut lapic = vcpu.fd.get_lapic().expect("get_lapic failed");
            let lvt0_bytes = unsafe {
                let p = &lapic.regs[APIC_LVT0..APIC_LVT0 + 4] as *const [i8] as *const [u8];
                *(&*p as *const [u8] as *const [u8; 4])
            };
            let mut lvt0 = u32::from_le_bytes(lvt0_bytes);
            lvt0 &= !(1 << 16); // clear Mask bit → unmask
            let updated = lvt0.to_le_bytes();
            unsafe {
                let dst = &mut lapic.regs[APIC_LVT0..APIC_LVT0 + 4] as *mut [i8] as *mut [u8];
                (&mut *dst).write(&updated).unwrap();
            }
            vcpu.fd.set_lapic(&lapic).expect("set_lapic failed");
        }

        let mut this = Self {
            vcpu,
            vm: Arc::clone(&vm),
            io_map: Arc::clone(&io_map),
            mmio_map: Arc::clone(&mmio_map),
            memory_regions: Arc::clone(&guest_memory),
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

                    this.memory_regions
                        .lock()
                        .unwrap()
                        .last()
                        .unwrap()
                        .write(binary.data.as_mut_slice(), code_offset);
                }
            }
        }

        for mut mmio_device in machine_config.mmio_devices {
            mmio_device.irq_handler(Arc::clone(&irq_handler));
            mmio_device.vm_fd(Arc::clone(&vm));
            mmio_device.pass_guest_memory(Arc::clone(&guest_memory));
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

        this.tick(TickContext::new(
            io_map_tick,
            mmio_map_tick,
            irq_handler_tick,
            vm_tick,
        ));

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
        self.memory_regions.lock().unwrap().push(MemoryRegion::new(
            userspace_mem,
            mem_size,
            mem_offset,
        ));

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
}
