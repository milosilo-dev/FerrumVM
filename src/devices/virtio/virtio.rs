use std::cell::Cell;
use std::sync::atomic::{Ordering, fence};

use crate::memory_region::GuestMemoryHandle;

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
    pub last_written_used_idx: Cell<u16>,
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
            last_written_used_idx: Cell::new(0),
        }
    }

    pub fn pop_avail(&mut self, mem: &VirtioGuestMemoryHandle) -> Option<u16> {
        let avail_idx = mem.read_u16(self.avail_addr + 2);

        let entries_available = avail_idx.wrapping_sub(self.last_avail_idx);
        if entries_available == 0 {
            return None;
        }

        let ring_offset = 4 + (self.last_avail_idx % self.size) as u64 * 2;
        let head = mem.read_u16(self.avail_addr + ring_offset);

        if head >= self.size {
            self.last_avail_idx = avail_idx;
            return None;
        }

        self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
        Some(head)
    }

    pub fn push_used(&self, mem: &mut VirtioGuestMemoryHandle, head: u16, len: u32) {
        let used_idx = mem.read_u16(self.used_addr + 2);

        let offset = 4 + (used_idx % self.size) as u64 * 8;

        mem.write_u32(self.used_addr + offset, head as u32);
        mem.write_u32(self.used_addr + offset + 4, len);

        fence(Ordering::SeqCst);

        let new_idx = used_idx.wrapping_add(1);
        mem.write_u16(self.used_addr + 2, new_idx);

        fence(Ordering::SeqCst);

        let verify = mem.read_u16(self.used_addr + 2);
        debug_assert!(
            verify == new_idx,
            "push_used verify FAILED: wrote {} read back {}",
            new_idx,
            verify
        );
        self.last_written_used_idx.set(new_idx);
    }

    pub fn get_descriptor(&self, mem: &VirtioGuestMemoryHandle, index: u16) -> VirtqDesc {
        let desc_addr = self.desc_addr + (index as u64) * 16;

        VirtqDesc {
            addr: mem.read_u64(desc_addr + 0),
            len: mem.read_u32(desc_addr + 8),
            flags: mem.read_u16(desc_addr + 12),
            next: mem.read_u16(desc_addr + 14),
        }
    }

    pub fn read_avail_entry(&self, mem: &VirtioGuestMemoryHandle, idx: u16) -> u16 {
        let ring_offset = 4 + ((idx % self.size) as u64) * 2;

        mem.read_u16(self.avail_addr + ring_offset)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory_region::MemoryRegion;
    use std::sync::{Arc, Mutex};

    #[test]
    fn test_push_used_updates_guest_memory() {
        let queue_size = 16;
        let used_ring_size = 4 + 2 + queue_size as usize * 8; // flags(2) + idx(2) + ring entries(8 each)
        let mut backing = vec![0u8; used_ring_size + 64];
        let ptr = backing.as_mut_ptr();
        let mem_offset = 0x1000u64;

        let mem_region = MemoryRegion::new(ptr, backing.len(), mem_offset);
        let handle = Arc::new(Mutex::new(vec![mem_region]));
        let mut vmem = VirtioGuestMemoryHandle::new(handle);

        let used_addr = mem_offset;

        vmem.write_u16(used_addr + 2, 0); // idx = 0
        vmem.write_u16(used_addr, 0); // flags = 0

        let queue = VirtioQueue {
            size: queue_size as u16,
            ready: true,
            desc_addr: 0,
            avail_addr: 0,
            used_addr,
            last_avail_idx: 0,
            last_written_used_idx: Cell::new(0),
        };

        queue.push_used(&mut vmem, 3, 64);

        let idx = vmem.read_u16(used_addr + 2);
        assert_eq!(idx, 1, "used idx should be 1 after first push");
        assert_eq!(
            queue.last_written_used_idx.get(),
            1,
            "Cell tracking should match"
        );

        let entry_id = vmem.read_u32(used_addr + 4);
        let entry_len = vmem.read_u32(used_addr + 8);
        assert_eq!(entry_id, 3, "first ring entry id should be head=3");
        assert_eq!(entry_len, 64, "first ring entry len should be 64");

        queue.push_used(&mut vmem, 7, 128);

        let idx = vmem.read_u16(used_addr + 2);
        assert_eq!(idx, 2, "used idx should be 2 after second push");
        assert_eq!(
            queue.last_written_used_idx.get(),
            2,
            "Cell tracking should match"
        );

        let entry_id = vmem.read_u32(used_addr + 4 + 8);
        let entry_len = vmem.read_u32(used_addr + 4 + 8 + 4);
        assert_eq!(entry_id, 7, "second ring entry id should be head=7");
        assert_eq!(entry_len, 128, "second ring entry len should be 128");

        for i in 0..queue_size * 2 {
            queue.push_used(&mut vmem, i as u16, (i * 10) as u32);
        }

        let idx = vmem.read_u16(used_addr + 2);
        assert_eq!(idx, 2 + queue_size as u16 * 2, "idx after wrap-around");
        assert_eq!(queue.last_written_used_idx.get(), idx);
    }
}
