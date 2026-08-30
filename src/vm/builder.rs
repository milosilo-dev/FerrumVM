use std::sync::{Arc, Mutex};

use kvm_bindings::{
    KVM_IRQ_ROUTING_IRQCHIP, kvm_irq_routing, kvm_irq_routing_entry, kvm_userspace_memory_region,
};
use kvm_ioctls::{Kvm, VmFd};
use libc::{MAP_ANONYMOUS, MAP_PRIVATE, PROT_READ, PROT_WRITE, mmap};
use vmm_sys_util::fam::FamStructWrapper;

use crate::{
    device_maps::{
        io::{IODeviceMap, IODeviceRegion},
        mmio::{MMIODeviceMap, MMIODeviceRegion},
    }, irq::handler::IRQHandler, machine_config::{machine_config::MachineConfig, memory_region::MemoryRegion}, vcpu::VCPU, vm::{tick::TickContext, vm::VirtualMachine},
};

impl VirtualMachine {
    pub fn new(mut machine_config: MachineConfig) -> Self {
        let kvm: Kvm = Kvm::new().unwrap();
        let vm = Arc::new(Mutex::new(kvm.create_vm().unwrap()));
        let _ = vm.lock().unwrap().create_irq_chip().unwrap();

        let mut routing: FamStructWrapper<kvm_irq_routing> =
            FamStructWrapper::new(machine_config.irq_map.len()).unwrap();

        let mut idx = 0;
        for irq_map in machine_config.irq_map.as_slice() {
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

        let mut vcpus: Vec<Arc<Mutex<VCPU>>> = vec![];
        for vcpu_id in 0..machine_config.total_vcpus{
            vcpus.push(Arc::new(Mutex::new(Self::new_vcpu(&kvm, &vm, &machine_config, vcpu_id))));
        }

        let mut this = Self {
            vcpus: vcpus,
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

    fn new_vcpu(kvm: &Kvm, vm: &Arc<Mutex<VmFd>>, machine_config: &MachineConfig, vcpu_id: u8) -> VCPU {
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

                1 => {
                    entry.ebx =
                        (entry.ebx & 0x00FF_FFFF) | ((vcpu_id as u32) << 24); // APIC ID
                }

                _ => {}
            }
        }

        let vcpu = VCPU::new(Arc::clone(&vm), vcpu_id as u64, machine_config.code_entry, &mut cpuid);

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
                (&mut *dst).write_all(&updated).unwrap();
            }
            vcpu.fd.set_lapic(&lapic).expect("set_lapic failed");
        }
        vcpu
    }

    /// APIC MMIO ranges that KVM's in-kernel irqchip (created via `create_irq_chip`)
    /// emulates. If a user memory slot covers these addresses, the accesses are routed
    /// to the slot (anonymous RAM) instead of the in-kernel IOAPIC/LAPIC, breaking
    /// interrupt delivery. We therefore never register a RAM slot over them.
    const IOAPIC_START: u64 = 0x0000_0000_0FEC0_0000;
    const IOAPIC_END: u64 = 0x0000_0000_0FEC0_1000;
    const LAPIC_START: u64 = 0x0000_0000_0FEE0_0000;
    const LAPIC_END: u64 = 0x0000_0000_0FEE0_1000;

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
        // Keep a single logical memory region so the guest still sees its RAM as one
        // contiguous block (used for DMA access via VirtioGuestMemoryHandle). The KVM
        // slot registration below is what must avoid the APIC hole.
        self.memory_regions.lock().unwrap().push(MemoryRegion::new(
            userspace_mem,
            mem_size,
            mem_offset,
        ));

        let vm_lock = self.vm.lock().unwrap();
        // Each logical region registered via `new_mem` becomes one or more contiguous
        // KVM slots. The IRQ chip's internal APIC pages use separate internal slots, so
        // user slots start at 0.
        let mut slot = 0u32;

        let start = mem_offset;
        let end = mem_offset + mem_size as u64;

        // Carve the LAPIC/IOAPIC hole out of the RAM so the in-kernel irqchip owns
        // 0xFEC00000 and 0xFEE00000. The hole can only exist below 4GB (KVM's in-kernel
        // APIC is a 32-bit-mapped device), so only split when the range overlaps it.
        type Range = (u64, u64);
        let mut segments: Vec<Range> = vec![(start, end)];

        let carve = [
            (Self::IOAPIC_START, Self::IOAPIC_END),
            (Self::LAPIC_START, Self::LAPIC_END),
        ];

        for (hole_start, hole_end) in carve {
            let mut next: Vec<Range> = Vec::new();
            for (seg_start, seg_end) in segments {
                if hole_start >= seg_end || hole_end <= seg_start {
                    next.push((seg_start, seg_end));
                } else {
                    if seg_start < hole_start {
                        next.push((seg_start, hole_start));
                    }
                    if hole_end < seg_end {
                        next.push((hole_end, seg_end));
                    }
                }
            }
            segments = next;
        }

        for (seg_start, seg_end) in segments {
            let memory_region = kvm_userspace_memory_region {
                slot,
                flags: 0,
                guest_phys_addr: seg_start,
                memory_size: seg_end - seg_start,
                userspace_addr: userspace_mem as u64 + (seg_start - start),
            };
            unsafe {
                vm_lock.set_user_memory_region(memory_region).unwrap();
            }
            slot += 1;
        }
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
