use std::{
    collections::VecDeque,
    fs::File,
    io::{self, Write},
    sync::{Arc, Mutex},
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{
    device_maps::io::IODevice,
    irq::handler::{IRQCommand, IRQHandler},
};

pub enum SerialMode {
    Terminal,
    LogFile(File),
}

pub struct Serial {
    data: VecDeque<u8>,
    irq_handler: Option<Arc<Mutex<IRQHandler>>>,
    queue: Arc<Mutex<Vec<u8>>>,
    mode: SerialMode,
    ier: u8,
    iir: u8,
    fcr: u8,
    lcr: u8,
    mcr: u8,
    lsr: u8,
    msr: u8,
    scr: u8,
    dll: u8,
    dlh: u8,
}

impl Serial {
    pub fn new(mode: SerialMode) -> Self {
        let queue = Arc::new(Mutex::new(Vec::<u8>::new()));

        if let SerialMode::Terminal = mode {
            enable_raw_mode().unwrap();
            std::thread::spawn({
                let queue = queue.clone();

                move || loop {
                    match event::read().unwrap() {
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('z'),
                            kind: KeyEventKind::Press,
                            modifiers: KeyModifiers::CONTROL,
                            ..
                        }) => {
                            disable_raw_mode().unwrap();
                            println!("");
                            std::process::exit(0);
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Char('w'),
                            kind: KeyEventKind::Press,
                            ..
                        }) => {
                            queue.lock().unwrap().push('\n' as u8);
                        }
                        Event::Key(KeyEvent {
                            code: KeyCode::Char(c),
                            kind: KeyEventKind::Press,
                            ..
                        }) => {
                            queue.lock().unwrap().push(c as u8);
                        }
                        _ => {}
                    }
                }
            });
        }

        Self {
            data: vec![].into(),
            irq_handler: None,
            queue,
            mode,
            ier: 0,
            iir: 0x01,
            fcr: 0,
            lcr: 0,
            mcr: 0,
            lsr: 0x60,
            msr: 0,
            scr: 0,
            dll: 0,
            dlh: 0,
        }
    }

    pub fn set_data(&mut self, new_data: Vec<u8>) {
        self.data.extend(new_data.iter());
        if self.irq_handler.is_some() {
            let irq_handler = self.irq_handler.as_mut().unwrap();
            irq_handler
                .lock()
                .unwrap()
                .trigger_irq(IRQCommand::new(4, true));
        }
    }

    fn update_iir(&mut self) {
        if self.fcr & 0x01 != 0 {
            self.iir |= 0xC0;
        } else {
            self.iir &= !0xC0;
        }
        let pending = if self.ier & 0x01 != 0 && !self.data.is_empty() {
            true
        } else {
            false
        };
        if pending {
            self.iir &= !0x01;
            self.iir = (self.iir & 0xF1) | (0x04 << 1);
        } else {
            self.iir |= 0x01;
        }
    }

    fn update_lsr(&mut self) {
        self.lsr |= 0x60;
        if !self.data.is_empty() {
            self.lsr |= 0x01;
        } else {
            self.lsr &= !0x01;
        }
    }
}

impl IODevice for Serial {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        let dlab = self.lcr & 0x80 != 0;
        match port {
            0 => {
                if dlab {
                    return vec![self.dll; length];
                }

                let mut out = vec![0; length];
                for i in 0..length {
                    let next_byte = self.data.pop_front();
                    if next_byte.is_some() {
                        out[i] = next_byte.unwrap();
                    }
                }
                out
            }
            1 => {
                if dlab {
                    return vec![self.dlh; length];
                }
                vec![self.ier; length]
            }
            2 => vec![self.iir; length],
            3 => vec![self.lcr; length],
            4 => vec![self.mcr; length],
            5 => {
                self.update_lsr();
                vec![self.lsr; length]
            }
            6 => vec![self.msr; length],
            7 => vec![self.scr; length],
            _ => vec![0; length],
        }
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        let dlab = self.lcr & 0x80 != 0;
        let val = data[0];
        match port {
            0 => {
                if dlab {
                    self.dll = val;
                    return;
                }
                if let SerialMode::Terminal = &mut self.mode {
                    print!("{}", val as char);
                    io::stdout().flush().unwrap();
                }
                if let SerialMode::LogFile(file) = &mut self.mode {
                    let _ = file.write(data);
                }
            }
            1 => {
                if dlab {
                    self.dlh = val;
                    return;
                }
                self.ier = val & 0x0F;
                self.update_iir();
            }
            2 => {
                self.fcr = val;
                if val & 0x01 != 0 {
                    self.iir |= 0xC0;
                } else {
                    self.iir &= !0xC0;
                }
                self.update_iir();
            }
            3 => {
                self.lcr = val;
            }
            4 => {
                self.mcr = val;
            }
            5 => {}
            6 => {}
            7 => {
                self.scr = val;
            }
            _ => {}
        }
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_handler = Some(irq_handler);
    }

    fn tick(&mut self) {
        {
            let mut queue = self.queue.lock().unwrap();
            while let Some(b) = queue.pop() {
                self.data.push_back(b);
            }
        }
        self.update_iir();
        self.update_lsr();
    }
}