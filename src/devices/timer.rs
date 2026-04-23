use crate::{
    device_maps::io::IODevice,
    irq::handler::{IRQCommand, IRQHandler},
};
use std::{
    sync::{Arc, Mutex},
    time::Instant,
};

const PIT_BASE_HZ: f64 = 1_193_182.0;

pub struct Pit {
    irq_handler: Option<Arc<Mutex<IRQHandler>>>,
    irq_line: u32,

    // command register fields
    access_mode: u8, // 1=lobyte, 2=hibyte, 3=lobyte/hibyte
    op_mode: u8,     // 2=rate gen, 3=square wave etc.

    // divisor programming
    divisor: u32,   // 0 treated as 65536
    latch_lo: bool, // expecting lo byte next?

    // tick accumulator
    // we accumulate fractional ticks in fixed point
    // tick() is called every 1ms = 1,000,000 ns
    acc_ns: f64,
    period_ns: f64,

    last_tick: Instant,
    programmed: bool,
}

impl Pit {
    pub fn new() -> Self {
        let divisor = 65536u32;
        Self {
            irq_handler: None,
            irq_line: 0,
            access_mode: 3,
            op_mode: 3,
            divisor,
            latch_lo: true,
            acc_ns: 0.0,
            period_ns: Self::period_ns(divisor),
            last_tick: Instant::now(),
            programmed: false,
        }
    }

    fn period_ns(divisor: u32) -> f64 {
        let d = if divisor == 0 { 65536 } else { divisor } as f64;
        (d / PIT_BASE_HZ) * 1_000_000_000.0
    }

    fn update_period(&mut self) {
        self.period_ns = Self::period_ns(self.divisor);
    }
}

impl IODevice for Pit {
    // called on guest `out port, al`
    fn output(&mut self, port: u16, data: &[u8]) {
        let val = data[0];
        match port {
            0x3 => {
                // mode/command register
                let channel = (val >> 6) & 0x3;
                if channel != 0 {
                    return; // only channel 0 supported
                }
                self.access_mode = (val >> 4) & 0x3;
                self.op_mode = (val >> 1) & 0x7;
                self.latch_lo = true;
                // reset divisor when reprogrammed
                self.divisor = 0;
            }
            0x0 => {
                // channel 0 divisor byte(s)
                match self.access_mode {
                    1 => {
                        // lobyte only
                        self.divisor = val as u32;
                        self.update_period();
                        self.programmed = true;
                    }
                    2 => {
                        // hibyte only
                        self.divisor = (val as u32) << 8;
                        self.update_period();
                        self.programmed = true;
                    }
                    3 => {
                        // lobyte then hibyte
                        if self.latch_lo {
                            self.divisor = val as u32;
                            self.latch_lo = false;
                        } else {
                            self.divisor |= (val as u32) << 8;
                            self.latch_lo = true;
                            self.update_period(); // fully programmed now
                            self.programmed = true;
                        }
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    }

    fn input(&mut self, _port: u16, len: usize) -> Vec<u8> {
        vec![0; len]
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_handler = Some(irq_handler);
    }

    fn tick(&mut self) {
        if self.irq_handler.is_none() || self.period_ns == 0.0 || !self.programmed {
            return;
        }

        // tick() called every 1ms
        self.acc_ns += self.last_tick.elapsed().as_nanos() as f64;
        self.last_tick = Instant::now();

        while self.acc_ns >= self.period_ns {
            self.acc_ns -= self.period_ns;

            let irq_arc = self.irq_handler.as_mut().unwrap();
            let mut handler = irq_arc.lock().unwrap();
            handler.trigger_irq(IRQCommand::new(self.irq_line, true));
            handler.trigger_irq(IRQCommand::new(self.irq_line, false));
        }
    }
}
