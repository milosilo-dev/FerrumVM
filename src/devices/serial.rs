use std::{collections::VecDeque, io::{self, Write}, sync::{Arc, Mutex}};

use crate::{device_maps::io::IODevice, irq_handler::{IRQCommand, IRQHandler}};

pub struct Serial{
    data: VecDeque<u8>,
    new_data: bool,
    irq_handler: Option<Arc<Mutex<IRQHandler>>>
}

impl Serial {
    pub fn new() -> Self {
        Self{
            data: vec![].into(),
            new_data: false,
            irq_handler: None,
        }
    }

    pub fn set_data(&mut self, new_data: Vec<u8>) {
        self.data = new_data.into();
        self.new_data = true;
        if self.irq_handler.is_some() {
            let irq_handler = self.irq_handler.as_mut().unwrap();
            irq_handler.lock().unwrap().trigger_irq(IRQCommand::new(4, true));
        }
    }
}

impl IODevice for Serial {
    fn input(&mut self, port: u16, length: usize) -> Vec<u8> {
        match port{
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
                let status = if self.new_data {0x01} else {0x20};
                vec![status; length]
            }
            _ => {vec![0; length]}
        }
    }

    fn output(&mut self, port: u16, data: &[u8]) {
        match port{
            0 => {
                for i in 0..data.len(){
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
    
    fn tick(&mut self) {}
}