use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use kvm_ioctls::VmFd;

use crate::{
    device_maps::{io::IODeviceMap, mmio::MMIODeviceMap},
    irq::handler::IRQHandler,
    vm::vm::VirtualMachine,
};

pub struct TickContext {
    io_map_tick: Arc<Mutex<IODeviceMap>>,
    mmio_map_tick: Arc<Mutex<MMIODeviceMap>>,
    irq_handler_tick: Arc<Mutex<IRQHandler>>,
    vm_tick: Arc<Mutex<VmFd>>,
}

impl TickContext {
    pub fn new(
        io_map_tick: Arc<Mutex<IODeviceMap>>,
        mmio_map_tick: Arc<Mutex<MMIODeviceMap>>,
        irq_handler_tick: Arc<Mutex<IRQHandler>>,
        vm_tick: Arc<Mutex<VmFd>>,
    ) -> Self {
        Self {
            io_map_tick,
            mmio_map_tick,
            irq_handler_tick,
            vm_tick,
        }
    }
}

impl VirtualMachine {
    pub fn tick(&mut self, context: TickContext) {
        thread::spawn(move || {
            loop {
                context.mmio_map_tick.lock().unwrap().tick();
                context.io_map_tick.lock().unwrap().tick();

                let mut irqs = {
                    let mut handler = context.irq_handler_tick.lock().unwrap();
                    handler.handle_irqs()
                };
                while let Some(irq) = irqs.pop_front() {
                    let vm_lock = context.vm_tick.lock().unwrap();
                    match vm_lock.set_irq_line(irq.irq_line, irq.value) {
                        Ok(_) => {}
                        Err(e) => println!("IRQ failed: {:?}", e),
                    }
                }

                thread::sleep(Duration::from_micros(100));
            }
        });
    }
}
