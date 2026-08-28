use std::sync::{Arc, Mutex};

use kvm_bindings::{kvm_cpuid2, kvm_mp_state, kvm_regs, kvm_segment, KVM_MP_STATE_UNINITIALIZED};
use kvm_ioctls::{VcpuFd, VmFd};
use vmm_sys_util::fam::FamStructWrapper;

pub struct VCPU {
    pub fd: VcpuFd,
}

fn real_mode_code_seg(base: u64, selector: u16) -> kvm_segment {
    kvm_segment {
        base,
        limit: 0xFFFF,
        selector,
        type_: 0xA, // execute/read
        present: 1,
        dpl: 0,
        db: 0,
        s: 1,
        l: 0,
        g: 0,
        avl: 0,
        unusable: 0,
        padding: 0,
    }
}

fn real_mode_data_seg(base: u64, selector: u16) -> kvm_segment {
    kvm_segment {
        base,
        limit: 0xFFFF,
        selector,
        type_: 0x2, // read/write
        present: 1,
        dpl: 0,
        db: 0,
        s: 1,
        l: 0,
        g: 0,
        avl: 0,
        unusable: 0,
        padding: 0,
    }
}

fn filter_cpuid(cpuid: &mut FamStructWrapper<kvm_cpuid2>) {
    for entry in cpuid.as_mut_slice() {
        match entry.function {
            0x1 => {
                // CPUID.1:EDX[16] = PAT
                entry.edx &= !(1 << 16);
            }
            _ => {}
        }
    }
}

impl VCPU {
    pub fn new(
        vm: Arc<Mutex<VmFd>>,
        vcpu_id: u64,
        entry: usize,
        mut cpuid: &mut FamStructWrapper<kvm_bindings::kvm_cpuid2>,
    ) -> Self {
        let vm_lock = vm.lock().unwrap();
        let vcpu = vm_lock.create_vcpu(vcpu_id).unwrap();

        filter_cpuid(&mut cpuid);
        let _ = vcpu.set_cpuid2(cpuid).unwrap();

        let mut sregs = vcpu.get_sregs().unwrap();

        sregs.cr0 = (sregs.cr0 & !0x1) | 0x20;
        sregs.cr4 = (1 << 9) | (1 << 10);

        sregs.cs = real_mode_code_seg(0, 0);
        sregs.ds = real_mode_data_seg(0, 0);
        sregs.es = real_mode_data_seg(0, 0);
        sregs.fs = real_mode_data_seg(0, 0);
        sregs.gs = real_mode_data_seg(0, 0);
        sregs.ss = real_mode_data_seg(0, 0);

        vcpu.set_sregs(&sregs).unwrap();

        if vcpu_id == 0 {
            let mut regs = kvm_regs::default();

            regs.rip = entry as u64;
            regs.rsp = 0x0FF0;
            regs.rsi = 0x20000;
            regs.rflags = 0x202;

            vcpu.set_regs(&regs).unwrap();
        } else {
            vcpu.set_mp_state(kvm_mp_state {
                mp_state: KVM_MP_STATE_UNINITIALIZED,
            }).unwrap();
        }

        Self { fd: vcpu }
    }
}
