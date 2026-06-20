use std::{
    io::{self, Write},
    sync::{Arc, Mutex},
};

use crate::{
    device_maps::mmio::MMIODevice,
    devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue},
    irq::handler::{IRQCommand, IRQHandler},
    memory_region::GuestMemoryHandle,
};

const MAGIC_NUMBER: u32 = 0x74726976;
const VERSION: u32 = 0x2;
const VENDOR_ID: u32 = 0x56484B53;
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

    device_features_sel: u32,
    driver_features_sel: u32,
    driver_features: u64,

    irq_line: Option<Arc<Mutex<IRQHandler>>>,
    irq_sel: u32,
}

const VIRTIO_F_VERSION_1: u64 = 1 << 32;

impl MMIOTransport {
    pub fn new(device: Box<dyn VirtioDevice + Send>, queue_num: usize, irq_sel: u32) -> Self {
        Self {
            device,
            queues: vec![VirtioQueue::new(); queue_num],
            queue_sel: 0,
            status: 0,
            interrupt_status: 0,
            device_features_sel: 0,
            driver_features_sel: 0,
            driver_features: 0,
            irq_line: None,
            irq_sel,
        }
    }
}

impl MMIODevice for MMIOTransport {
    fn read(&mut self, addr: u64, length: usize) -> Vec<u8> {
        if addr >= 0x100 {
            io::stdout().flush().unwrap();

            let offset = (addr - 0x100) as usize;
            let cfg_bytes = self.device.read_config(offset + length);

            return cfg_bytes[offset..offset + length].to_vec();
        }

        let value = (match addr {
            0x000 => MAGIC_NUMBER,
            0x004 => VERSION,
            0x008 => self.device.virtio_type(),
            0x00C => VENDOR_ID,
            0x010 => {
                let features = self.device.features() as u64 | VIRTIO_F_VERSION_1;
                if self.device_features_sel == 0 {
                    features as u32
                } else {
                    (features >> 32) as u32
                }
            }
            0x034 => QUEUE_NUM_MAX,
            0x038 => self.queues[self.queue_sel].size as u32,
            0x044 => self.queues[self.queue_sel].ready as u32,
            0x070 => self.status,
            0x060 => self.interrupt_status,
            _ => 0,
        } as u64)
            .to_le_bytes();

        value[..length].to_vec()
    }

    fn write(&mut self, addr: u64, data: &[u8]) {
        match addr {
            0x014 => self.device_features_sel = read_u32_from_data(data),
            0x020 => {
                let val = read_u32_from_data(data) as u64;
                if self.driver_features_sel == 0 {
                    self.driver_features =
                        (self.driver_features & 0xFFFFFFFF00000000) | val;
                } else {
                    self.driver_features =
                        (self.driver_features & 0x00000000FFFFFFFF) | (val << 32);
                }
            }
            0x024 => {
                self.driver_features_sel = read_u32_from_data(data);
            }
            0x028 => {}
            0x030 => {
                let sel = read_u32_from_data(data) as usize;
                if sel < self.queues.len() {
                    self.queue_sel = sel;
                }
            }
            0x038 => self.queues[self.queue_sel].size = u16::from_le_bytes([data[0], data[1]]),
            0x044 => {
                let was_ready = self.queues[self.queue_sel].ready;
                self.queues[self.queue_sel].ready = data[0] != 0;
                if !was_ready && data[0] != 0 {
                    self.queues[self.queue_sel].last_avail_idx = 0;
                }
            }
            0x050 => {
                let queue_idx = read_u32_from_data(data) as usize;

                if self.device.virtio_type() == 0x1 {
                    eprintln!("[notify] queue_idx={} size={} ready={}", queue_idx, self.queues[queue_idx].size, self.queues[queue_idx].ready);
                }
                if queue_idx < self.queues.len() && self.queues[queue_idx].ready {
                    let was_pending = self.interrupt_status != 0;
                    if self.device.as_mut().tick(queue_idx, &mut self.queues[queue_idx]) {
                        self.interrupt_status |= 1;
                    }
                    let now_pending = self.interrupt_status != 0;
                    if now_pending && !was_pending {
                        if let Some(ref irq_line) = self.irq_line {
                            irq_line
                                .lock()
                                .unwrap()
                                .trigger_irq(IRQCommand::new(self.irq_sel, true));
                        }
                    }
                }
            }
            0x060 | 0x064 => {
                let ack = read_u32_from_data(data);
                self.interrupt_status &= !ack;
                if self.interrupt_status == 0 {
                    if let Some(ref irq_line) = self.irq_line {
                        irq_line
                            .lock()
                            .unwrap()
                            .trigger_irq(IRQCommand::new(self.irq_sel, false));
                    }
                }
            }
            0x070 => {
                let val = read_u32_from_data(data);
                if val == 0 {
                    for q in &mut self.queues {
                        *q = VirtioQueue::new();
                    }
                    self.interrupt_status = 0;
                }
                self.status = val;
            }
            0x080 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].desc_addr =
                    (self.queues[self.queue_sel].desc_addr & 0xFFFFFFFF00000000) | val;
            }
            0x084 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].desc_addr =
                    (self.queues[self.queue_sel].desc_addr & 0x00000000FFFFFFFF) | (val << 32);
            }
            0x090 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].avail_addr =
                    (self.queues[self.queue_sel].avail_addr & 0xFFFFFFFF00000000) | val;
            }
            0x094 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].avail_addr =
                    (self.queues[self.queue_sel].avail_addr & 0x00000000FFFFFFFF) | (val << 32);
            }
            0x0A0 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].used_addr =
                    (self.queues[self.queue_sel].used_addr & 0xFFFFFFFF00000000) | val;
            }
            0x0A4 => {
                let val = read_u32_from_data(data) as u64;
                self.queues[self.queue_sel].used_addr =
                    (self.queues[self.queue_sel].used_addr & 0x00000000FFFFFFFF) | (val << 32);
            }
            _ => {}
        }
    }

    fn pass_guest_memory(&mut self, guest_memory: GuestMemoryHandle) {
        self.device
            .pass_guest_memory(VirtioGuestMemoryHandle::new(guest_memory));
    }

    fn tick(&mut self) {
        if self.status & 4 == 0 {
            return;
        }

        let was_pending = self.interrupt_status != 0;

        for (idx, queue) in &mut self.queues.iter_mut().enumerate() {
            if queue.ready {
                let completions = self.device.as_mut().tick(idx, queue);
                if completions {
                    self.interrupt_status |= 1;
                }
            }
        }

        if self.device.update(self.queues.as_mut_slice()) {
            self.interrupt_status |= 1;
        }

        let now_pending = self.interrupt_status != 0;
        if now_pending && !was_pending {
            if let Some(ref irq_line) = self.irq_line {
                irq_line
                    .lock()
                    .unwrap()
                    .trigger_irq(IRQCommand::new(self.irq_sel, true));
            }
        }
    }

    fn irq_handler(&mut self, irq_handler: Arc<Mutex<IRQHandler>>) {
        self.irq_line = Some(irq_handler);
    }
}
