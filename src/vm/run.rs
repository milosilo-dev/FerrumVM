use kvm_ioctls::VcpuExit;
use std::io::Write;
use std::sync::Arc;
use crossterm::terminal::disable_raw_mode;

use crate::vm::vm::VirtualMachine;

#[derive(Debug)]
pub enum CrashReason {
    Hlt,
    FailedEntry,
    UnhandledExit,
    NoIODataReturned,
    IncorrectIOInputLength,
    NoMMIODataReturned,
    IncorrectMMIOReadLength,
    Shutdown,
    RunError,
}

impl VirtualMachine {
    pub fn run(&self, vcpu_id: usize) -> Result<(), CrashReason> {
        loop {
            let mut vcpu = match self.vcpus.get(vcpu_id).and_then(|v| v.lock().ok()) {
                Some(vcpu) => vcpu,
                None => return Err(CrashReason::RunError),
            };

            let exit = match vcpu.fd.run() {
                Ok(exit) => exit,

                Err(e) if e.errno() == libc::EINTR => {
                    continue;
                }

                Err(_) => return Err(CrashReason::RunError),
            };

            match exit {
                VcpuExit::Hlt => {
                    let regs = vcpu.fd.get_regs().ok();

                    if let Some(regs) = regs {
                        println!("KVM_EXIT_HLT at RIP={:#x}", regs.rip);
                    } else {
                        println!("KVM_EXIT_HLT");
                    }

                    std::io::stdout().flush().ok();

                    return Err(CrashReason::Hlt);
                }

                VcpuExit::IoOut(port, data) => {
                    if port == 0x500 {
                        println!("VM HALT via port 0x500");
                        return Err(CrashReason::Hlt);
                    }

                    let mut io_map = match self.io_map.lock() {
                        Ok(map) => map,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    io_map.output(port, data);
                }

                VcpuExit::IoIn(port, data) => {
                    let mut io_map = match self.io_map.lock() {
                        Ok(map) => map,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    if let Some(io_ret) = io_map.input(port, data.len()) {
                        if io_ret.len() != data.len() {
                            println!("INCORRECT_IO_INPUT_LENGTH");
                            return Err(CrashReason::IncorrectIOInputLength);
                        }

                        data.copy_from_slice(&io_ret);
                    } else {
                        data.fill(0xFF);
                    }
                }

                VcpuExit::MmioWrite(addr, data) => {
                    let mut mmio_map = match self.mmio_map.lock() {
                        Ok(map) => map,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    mmio_map.write(addr, data);
                }

                VcpuExit::MmioRead(addr, data) => {
                    let mut mmio_map = match self.mmio_map.lock() {
                        Ok(map) => map,
                        Err(poisoned) => poisoned.into_inner(),
                    };

                    if let Some(io_ret) = mmio_map.read(addr, data.len()) {
                        if io_ret.len() != data.len() {
                            println!("INCORRECT_MMIO_INPUT_LENGTH");
                            return Err(CrashReason::IncorrectMMIOReadLength);
                        }

                        data.copy_from_slice(&io_ret);
                    } else {
                        data.fill(0);
                    }
                }

                VcpuExit::FailEntry(reason, ..) => {
                    eprintln!(
                        "KVM_EXIT_FAIL_ENTRY: reason = {:#x}",
                        reason
                    );

                    return Err(CrashReason::FailedEntry);
                }

                VcpuExit::Shutdown => {
                    eprintln!("KVM_SHUTDOWN");

                    return Err(CrashReason::Shutdown);
                }

                exit_reason => {
                    println!("Unhandled exit: {:?}", exit_reason);
                    return Err(CrashReason::UnhandledExit);
                }
            }
        }
    }

    pub fn threaded_run(self: Arc<Self>) {
        let vcpu_count = self.vcpus.len();
        for vcpu_id in 0..vcpu_count {
            let vm = self.clone();
            std::thread::spawn(move || {
                loop {
                    let ret = vm.run(vcpu_id);
                    if let Err(reason) = ret {
                        disable_raw_mode().unwrap();
                        eprintln!("VCPU 0x{:X} crashed: {:?}\n", vcpu_id, reason);
                        panic!("VCPU 0x{:X} crashed!\n", vcpu_id);
                    }
                }
            });
        }
        loop {}
    }
}
