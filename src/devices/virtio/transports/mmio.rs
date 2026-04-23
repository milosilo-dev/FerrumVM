use std::sync::{Arc, Mutex};

use crate::{device_maps::mmio::MMIODevice, devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue}, irq_handler::{IRQCommand, IRQHandler}, memory_region::GuestMemoryHandle};

const MAGIC_NUMBER: u32 = 0x74726976;
const VERSION: u32 = 0x2;
const VENDOR_ID: u32 = 0x56484B53;
const IRQ_LINE: u32 = 5;
const QUEUE_NUM_MAX: u32 = 16;

fn read_u32_from_data(data: &[u8]) -> u32 {
    let mut buf = [0u8; 4];
    buf[..data.len()].copy_from_slice(data);
    u32::from_le_bytes(buf)
}

pub struct MMIOTransport {
    device: Box<dyn VirtioDevice + Send>,
    queues: Vec<VirtioQueue>,

    queue_sel: usize,
    status: u32,
    interrupt_status: u32,

    irq_line: Option<Arc<Mutex<IRQHandler>>>,
}

impl MMIOTransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize) -> Self{
        Self {
            device,
            queues: vec![VirtioQueue::new(); queue_num],
            queue_sel: 0,
            status: 0,
            interrupt_status: 0,
            irq_line: None,
        }
    }
}

impl MMIODevice for MMIOTransport {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        let value = (match addr {
            0x000 => MAGIC_NUMBER,
            0x004 => VERSION,
            0x008 => self.device.virtio_type(),
            0x00C => VENDOR_ID,
            0x010 => self.device.features(),
            0x034 => QUEUE_NUM_MAX,
            0x038 => self.queues[self.queue_sel].size as u32,
            0x044 => self.queues[self.queue_sel].ready as u32,
            0x070 => self.status,
            0x060 => self.interrupt_status,
            _ => 0,
        } as u64).to_le_bytes();
        value[..length].to_vec()
    }

    fn write(&mut self, addr: u64, data: &[u8]) {
        match addr{
            0x028 => {},
            0x030 => self.queue_sel = data[data.len() - 1] as usize,
            0x038 => self.queues[self.queue_sel].size = u16::from_le_bytes([data[0], data[1]]),
            0x044 => self.queues[self.queue_sel].ready = data[0] != 0,
            0x060 => {},
            0x070 => {
                self.status = read_u32_from_data(data);
            },
            0x080 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].desc_addr =
                    (self.queues[self.queue_sel].desc_addr & 0xFFFFFFFF00000000) | val;
            },
            0x084 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].desc_addr =
                    (self.queues[self.queue_sel].desc_addr & 0x00000000FFFFFFFF) | val;
            },
            0x090 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].avail_addr =
                    (self.queues[self.queue_sel].avail_addr & 0xFFFFFFFF00000000) | val;
            },
            0x094 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].avail_addr =
                    (self.queues[self.queue_sel].avail_addr & 0x00000000FFFFFFFF) | val;
            },
            0x0A0 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].used_addr =
                    (self.queues[self.queue_sel].used_addr & 0xFFFFFFFF00000000) | val;
            }
            0x0A4 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].used_addr =
                    (self.queues[self.queue_sel].used_addr & 0x00000000FFFFFFFF) | val;
            },
            _ => {}
        }
    }

    fn pass_guest_memory(&mut self, guest_memory: GuestMemoryHandle) {
        self.device.pass_guest_memory(VirtioGuestMemoryHandle::new(guest_memory));
    }

    fn tick(&mut self) {
        for queue in &mut self.queues{
            if queue.ready{
                let completions = self.device.as_mut().tick(queue);
                if completions && self.irq_line.is_some(){
                    self.irq_line
                        .as_mut()
                        .unwrap()
                        .lock()
                        .unwrap()
                        .trigger_irq(IRQCommand::new(IRQ_LINE, true));
                }
            }
        }
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_line = Some(irq_handler);
    }
}