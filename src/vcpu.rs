use std::sync::{Arc, Mutex};

use kvm_bindings::kvm_regs;
use kvm_ioctls::{VcpuFd, VmFd};

pub struct VCPU {
    pub fd: VcpuFd,
}

impl VCPU {
    pub fn new(vm: Arc<Mutex<VmFd>>, entry: usize) -> Self {
        let vm_lock = vm.lock().unwrap();
        let vcpu = vm_lock.create_vcpu(0).unwrap();

        let mut sregs = vcpu.get_sregs().unwrap();

        // real mode
        sregs.cr0 &= !0x1;

        // segments
        sregs.cs.base = 0;
        sregs.cs.selector = 0;
        sregs.cs.limit = 0xFFFF;

        sregs.ds.base = 0;
        sregs.ds.limit = 0xFFFF;

        sregs.es.base = 0;
        sregs.es.limit = 0xFFFF;

        sregs.fs.base = 0;
        sregs.fs.limit = 0xFFFF;

        sregs.gs.base = 0;
        sregs.gs.limit = 0xFFFF;

        sregs.ss.base = 0;
        sregs.ss.limit = 0xFFFF;

        vcpu.set_sregs(&sregs).unwrap();

        let mut regs = kvm_regs::default();
        regs.rip = entry as u64;
        regs.rsi = 0x10000;
        regs.rflags = 0x202;

        vcpu.set_regs(&regs).unwrap();

        Self { fd: vcpu }
    }
}
