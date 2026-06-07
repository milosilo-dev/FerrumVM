use libc;
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
    pub fn dump_mem(&self, addr: u64, len: usize) {
        let regions = self.memory_regions.lock().unwrap();
        for region in regions.iter() {
            let end = region.mem_offset + region.mem_size as u64;
            if addr >= region.mem_offset && addr < end {
                let offset = (addr - region.mem_offset) as usize;
                let available = (end - addr).min(len as u64) as usize;
                if let Some(data) = region.read(offset, available) {
                    print!("dump at {:#x}: ", addr);
                    for (i, b) in data.iter().enumerate().take(64) {
                        if i > 0 && i % 16 == 0 {
                            print!("\n{:16}", "");
                        }
                        print!("{:02x} ", b);
                    }
                    println!();
                }
                break;
            }
        }
    }

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
                eprintln!(
                    "RAX={:#x} RBX={:#x} RCX={:#x} RDX={:#x}",
                    regs.rax, regs.rbx, regs.rcx, regs.rdx
                );
                eprintln!(
                    "CR0={:#x} CR3={:#x} CR4={:#x} EFER={:#x}",
                    sregs.cr0, sregs.cr3, sregs.cr4, sregs.efer
                );
                eprintln!(
                    "CS base={:#x} selector={:#x} type={:#x} l={}",
                    sregs.cs.base, sregs.cs.selector, sregs.cs.type_, sregs.cs.l
                );
                return Err(CrashReason::Shutdown);
            }
            exit_reason => {
                println!("Unhandled exit: {:?}", exit_reason);
            }
        }
        Ok(())
    }
}
