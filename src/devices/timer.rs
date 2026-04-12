use std::{sync::{Arc, Mutex}, time::Instant};

use crate::{device_maps::mmio::MMIODevice, irq_handler::{IRQCommand, IRQHandler}};

pub struct Timer{
    irq_handler: Option<Arc<Mutex<IRQHandler>>>,
    interval: u32,
    enabled: bool,
    irq_line: u32,
    last_tick: Instant,
}

impl Timer{
    pub fn new() -> Self{
        Self{
            irq_handler: None,
            interval: 1000,
            enabled: true,
            irq_line: 1,
            last_tick: Instant::now(),
        }
    }
}

// MMIO Device to be mapped from 0xF0001000 - 0xF0001008
impl MMIODevice for Timer {
    fn read(&mut self, _addr: u64, _length: usize) -> Vec<u8> {
        vec![]
    }

    fn write(&mut self, addr: u64, data: &[u8]) {
        let value = u32::from_le_bytes(data.try_into().unwrap());
        println!("Wrote to timer: {}", value);

        match addr{
            0x0 => self.interval = value,
            0x4 => self.enabled = value != 0,
            0x8 => self.irq_line = value,
            _ => {}
        }
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_handler = Some(irq_handler);
    }

    fn tick(&mut self) {
        if !self.enabled || self.irq_handler.is_none() {
            return;
        }

        if self.last_tick.elapsed().as_millis() >= self.interval as u128 {
            let irq_arc = self.irq_handler.as_mut().unwrap();
            let mut irq_handler = irq_arc.lock().unwrap();

            irq_handler.trigger_irq(IRQCommand::new(self.irq_line, true));
            irq_handler.trigger_irq(IRQCommand::new(self.irq_line, false));

            self.last_tick = Instant::now();
        }
    }
}