use crate::{device_maps::mmio::MMIODevice, devices::virtio::virtio::{VirtioDevice, VirtioQueue}, memory_region::GuestMemoryHandle};

const MAGIC_NUMBER: u32 = 0x74726976;
const VERSION: u32 = 0x2;
const VENDOR_ID: u32 = 0x56484B53;

pub struct MMIOTransport {
    device: Box<dyn VirtioDevice + Sync + Send>,
    queues: Vec<VirtioQueue>,

    queue_sel: usize,
    status: u32,
    interrupt_status: u32,
}

impl MMIOTransport {
    pub fn new(device: Box<dyn VirtioDevice + Sync + Send>) -> Self{
        Self {
            device,
            queues: vec![],
            queue_sel: 0,
            status: 0,
            interrupt_status: 0,
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
            0x034 => self.queue_sel as u32,
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
            0x030 => self.queue_sel = data[data.len() - 1] as usize,
            0x038 => self.queues = vec![VirtioQueue::new(); data[data.len() - 1] as usize],
            0x044 => self.queues[self.queue_sel].ready = data[data.len() - 1] != 0,
            0x050 => self.queues[self.queue_sel].queue_notify(),
            0x080 => self.queues[self.queue_sel].desc_addr = (self.queues[self.queue_sel].desc_addr & 0xFF00) | (data[data.len() - 1] as u16),
            0x084 => self.queues[self.queue_sel].desc_addr = (self.queues[self.queue_sel].desc_addr & 0x00FF) | (data[data.len() - 1] as u16),
            0x090 => self.queues[self.queue_sel].avail_addr = (self.queues[self.queue_sel].avail_addr & 0xFF00) | (data[data.len() - 1] as u16),
            0x094 => self.queues[self.queue_sel].avail_addr = (self.queues[self.queue_sel].avail_addr & 0x00FF) | (data[data.len() - 1] as u16),
            0x0A0 => self.queues[self.queue_sel].used_addr = (self.queues[self.queue_sel].used_addr & 0xFF00) | (data[data.len() - 1] as u16),
            0x0A4 => self.queues[self.queue_sel].used_addr = (self.queues[self.queue_sel].used_addr & 0x00FF) | (data[data.len() - 1] as u16),
            _ => {}
        }
    }

    fn pass_guest_memory(&mut self, guest_memory: GuestMemoryHandle) {
        self.device.pass_guest_memory(guest_memory);
    }
}