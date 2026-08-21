use kvm_ioctls::VcpuExit;
use std::io::Write;

use crate::vm::vm::VirtualMachine;

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
    pub fn run(&mut self) -> Result<(), CrashReason> {
        let exit = loop {
            match self.vcpu.fd.run() {
                Ok(exit) => break exit,
                Err(e) if e.errno() == libc::EINTR => continue, // signal interrupted, retry
                Err(_) => return Err(CrashReason::RunError),
            }
        };

        match exit {
            VcpuExit::Hlt => {
                let regs = self.vcpu.fd.get_regs().ok();
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
                    for b in data.iter_mut() {
                        *b = 0xFF;
                    }
                    return Ok(());
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
                    for b in data.iter_mut() {
                        *b = 0;
                    }
                    return Ok(());
                }
            }
            VcpuExit::FailEntry(reason, ..) => {
                eprintln!("KVM_EXIT_FAIL_ENTRY: reason = {:#x}", reason);
                return Err(CrashReason::FailedEntry);
            }
            VcpuExit::Shutdown => {
                eprintln!("KVM_SHUTDOWN");
                return Err(CrashReason::Shutdown);
            }
            exit_reason => {
                println!("Unhandled exit: {:?}", exit_reason);
            }
        }
        Ok(())
    }
}
