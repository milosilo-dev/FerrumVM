use std::sync::atomic::{Ordering, fence};
use std::sync::{Arc, Mutex};

use crate::memory_region::{GuestMemoryHandle, MemoryRegion};

pub trait VirtioDevice {
    fn virtio_type(&self) -> u32;
    fn features(&self) -> u32;
    fn pass_guest_memory(&mut self, _guest_memory: VirtioGuestMemoryHandle);
    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool;
    fn read_config(&self, length: usize) -> Vec<u8>;
    fn write_config(&mut self, _offset: usize, _data: &[u8]) {}
    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool;
}

pub struct VirtioGuestMemoryHandle {
    mem: GuestMemoryHandle,
}

impl VirtioGuestMemoryHandle {
    pub fn new(mem: GuestMemoryHandle) -> Self {
        Self { mem }
    }

    pub fn memory_regions(&self) -> Arc<Mutex<Vec<MemoryRegion>>> {
        self.mem.clone()
    }

    pub fn read_byte(&self, addr: u64) -> u8 {
        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + 1 <= end {
                let data = mem_region
                    .read((addr - mem_region.mem_offset) as usize, 1 as usize)
                    .unwrap();
                return data[0];
            }
        }
        0
    }

    pub fn read_u16(&self, addr: u64) -> u16 {
        const LENGTH: u64 = 2;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region
                    .read((addr - mem_region.mem_offset) as usize, LENGTH as usize)
                    .unwrap();
                return u16::from_le_bytes([data[0], data[1]]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn read_u32(&self, addr: u64) -> u32 {
        const LENGTH: u64 = 4;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region
                    .read((addr - mem_region.mem_offset) as usize, LENGTH as usize)
                    .unwrap();
                return u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn read_u64(&self, addr: u64) -> u64 {
        const LENGTH: u64 = 8;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region
                    .read((addr - mem_region.mem_offset) as usize, LENGTH as usize)
                    .unwrap();
                return u64::from_le_bytes([
                    data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7],
                ]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn read_guest_memory(&self, addr: u64, buf: &mut Vec<u8>) {
        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + buf.len() as u64 <= end {
                let data = mem_region
                    .read((addr - mem_region.mem_offset) as usize, buf.len())
                    .unwrap();
                *buf = data;
                return;
            }
        }
    }

    pub fn write_u8(&mut self, addr: u64, val: u8) {
        const LENGTH: u64 = 1;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = &val.to_le_bytes();
                mem_region.write(data, (addr - mem_region.mem_offset) as usize);
                return;
            }
        }
        println!("Virtio wrote a addr outside of mapped scope!");
    }

    pub fn write_u16(&mut self, addr: u64, val: u16) {
        const LENGTH: u64 = 2;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = &val.to_le_bytes();
                mem_region.write(data, (addr - mem_region.mem_offset) as usize);
                return;
            }
        }
        println!("Virtio wrote a addr outside of mapped scope!");
    }

    pub fn write_u32(&mut self, addr: u64, val: u32) {
        const LENGTH: u64 = 4;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = &val.to_le_bytes();
                mem_region.write(data, (addr - mem_region.mem_offset) as usize);
                return;
            }
        }
        println!("Virtio wrote a addr outside of mapped scope!");
    }

    pub fn write_guest_memory(&mut self, addr: u64, data: &[u8]) {
        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + data.len() as u64 <= end {
                mem_region.write(data, (addr - mem_region.mem_offset) as usize);
                return;
            }
        }
        println!("Virtio wrote a addr outside of mapped scope!");
    }
}

#[derive(Clone, Copy)]
pub struct VirtqDesc {
    pub addr: u64,
    pub len: u32,
    pub flags: u16,
    pub next: u16,
}

#[derive(Clone)]
pub struct VirtioQueue {
    pub size: u16,
    pub ready: bool,

    pub desc_addr: u64,
    pub avail_addr: u64,
    pub used_addr: u64,

    pub last_avail_idx: u16,
    pub last_used_idx: u16,
}

impl VirtioQueue {
    pub fn new() -> Self {
        Self {
            size: 0,
            ready: false,
            desc_addr: 0,
            avail_addr: 0,
            used_addr: 0,
            last_avail_idx: 0,
            last_used_idx: 0,
        }
    }

    #[inline(always)]
    fn read_u16(mem: &VirtioGuestMemoryHandle, addr: u64) -> u16 {
        fence(Ordering::SeqCst);
        let v = mem.read_u16(addr);
        fence(Ordering::SeqCst);
        v
    }

    #[inline(always)]
    fn write_u16(mem: &mut VirtioGuestMemoryHandle, addr: u64, val: u16) {
        fence(Ordering::SeqCst);
        mem.write_u16(addr, val);
        fence(Ordering::SeqCst);
    }

    pub fn pop_avail(&mut self, mem: &VirtioGuestMemoryHandle) -> Option<u16> {
        if self.size == 0 {
            return None;
        }

        let avail_idx = Self::read_u16(mem, self.avail_addr + 2);

        // nothing new
        if self.last_avail_idx == avail_idx {
            return None;
        }

        let ring_idx = self.last_avail_idx % self.size;
        let offset = 4 + (ring_idx as u64) * 2;

        let head = Self::read_u16(mem, self.avail_addr + offset);

        self.last_avail_idx = self.last_avail_idx.wrapping_add(1);

        Some(head)
    }

    // -------------------------
    // USED RING (FIXED)
    // -------------------------

    pub fn push_used(&mut self, mem: &mut VirtioGuestMemoryHandle, head: u16, len: u32) {
        if self.size == 0 {
            return;
        }

        let idx = self.last_used_idx;

        let offset = 4 + (idx % self.size) as u64 * 8;

        mem.write_u32(self.used_addr + offset, head as u32);
        mem.write_u32(self.used_addr + offset + 4, len);

        self.last_used_idx = self.last_used_idx.wrapping_add(1);

        // publish to guest (IMPORTANT)
        Self::write_u16(mem, self.used_addr + 2, self.last_used_idx);
    }

    // -------------------------
    // DESCRIPTOR FETCH (UNCHANGED LOGIC, FIXED SAFETY)
    // -------------------------

    pub fn get_descriptor(&self, mem: &VirtioGuestMemoryHandle, index: u16) -> VirtqDesc {
        let base = self.desc_addr + (index as u64) * 16;

        VirtqDesc {
            addr: mem.read_u64(base),
            len: mem.read_u32(base + 8),
            flags: mem.read_u16(base + 12),
            next: mem.read_u16(base + 14),
        }
    }

    // optional helper
    pub fn read_avail_entry(&self, mem: &VirtioGuestMemoryHandle, idx: u16) -> u16 {
        let ring_offset = 4 + ((idx % self.size) as u64) * 2;
        mem.read_u16(self.avail_addr + ring_offset)
    }
}
