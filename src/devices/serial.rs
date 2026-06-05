use std::{
    collections::VecDeque, io::{self, Write}, sync::{Arc, Mutex}
};

use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};

use crate::{
    device_maps::io::IODevice,
    irq::handler::{IRQCommand, IRQHandler},
};

pub struct Serial {
    data: VecDeque<u8>,
    irq_handler: Option<Arc<Mutex<IRQHandler>>>,
    queue: Arc<Mutex<Vec<u8>>>,
}

impl Serial {
    pub fn new() -> Self {
        enable_raw_mode().unwrap();
        let queue = Arc::new(Mutex::new(Vec::<u8>::new()));

        std::thread::spawn({
            let queue = queue.clone();

            move || loop {
                match event::read().unwrap() {
                    Event::Key(KeyEvent { code: KeyCode::Char('z'), kind: KeyEventKind::Press, modifiers: KeyModifiers::CONTROL, .. }) => {
                        disable_raw_mode().unwrap();
                        println!("");
                        std::process::exit(0);
                    }
                    Event::Key(KeyEvent { code: KeyCode::Char(c), kind: KeyEventKind::Press, .. }) => {
                        queue.lock().unwrap().push(c as u8);
                    }
                    _ => {}
                }
            }
        });

        Self {
            data: vec![].into(),
            irq_handler: None,
            queue,
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
}

impl IODevice for Serial {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        match port {
            0 => {
                let mut out = vec![0; length];
                for i in 0..length {
                    let next_byte = self.data.pop_front();
                    if next_byte.is_some() {
                        out[i] = next_byte.unwrap();
                    }
                }
                out
            }
            5 => {
                let mut status = 0x20;

                if !self.data.is_empty() {
                    status |= 0x01;
                }
                vec![status; length]
            }
            _ => {
                vec![0; length]
            }
        }
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        match port {
            0 => {
                for i in 0..data.len() {
                    print!("{}", data[i] as char);
                }
                io::stdout().flush().unwrap();
            }
            _ => {}
        }
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_handler = Some(irq_handler);
    }

    fn tick(&mut self) {
        let mut queue = self.queue.lock().unwrap();
        while let Some(b) = queue.pop() {
            self.data.push_back(b);
        }
    }
}
