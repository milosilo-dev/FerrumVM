use kvm_bindings::kvm_regs;
use kvm_ioctls::{VcpuExit, VcpuFd, VmFd};

pub struct VCPU {
    pub fd: VcpuFd
}

impl VCPU{
    pub fn new(vm: VmFd) -> Self{
        let vcpu = vm.create_vcpu(0).unwrap();

        let mut sregs = vcpu.get_sregs().unwrap();
        sregs.cs.base = 0;
        sregs.cs.selector = 0;
        sregs.ds.base = 0;
        sregs.es.base = 0;
        sregs.fs.base = 0;
        sregs.gs.base = 0;
        sregs.ss.base = 0;
        vcpu.set_sregs(&sregs).unwrap();

        let mut regs = kvm_regs::default();
        regs.rip = 0x1000;
        regs.rax = 2;
        regs.rbx = 2;
        regs.rflags = 0x2;
        vcpu.set_regs(&regs).unwrap();

        Self {
            fd: vcpu 
        }
    }

    pub fn run(&mut self) -> VcpuExit<'_> {
        self.fd.run().expect("run failed")
    }
}