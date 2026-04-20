use crate::memory_region::{GuestMemoryHandle, MemoryRegion};

pub trait VirtioDevice {
    fn virtio_type(&self) -> u32;
    fn features(&self) -> u32;
    fn pass_guest_memory(&mut self, _guest_memory: GuestMemoryHandle) {}
}

pub struct VirtioGuestMemoryHandle {}

#[derive(Clone)]
pub struct VirtioQueue {
    pub size: u16,
    pub ready: bool,

    pub desc_addr: u16,
    pub avail_addr: u16,
    pub used_addr: u16,
    pub last_avail_idx: u16,
}

impl VirtioQueue {
    pub fn new() -> Self{
        Self{
            size: 0,
            ready: false,
            desc_addr: 0,
            avail_addr: 0,
            used_addr: 0,
            last_avail_idx: 0,
        }
    }

    pub fn queue_notify(&self) {
        todo!()
    }
}