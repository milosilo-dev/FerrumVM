use crate::memory_region::{GuestMemoryHandle};

pub trait VirtioDevice {
    fn virtio_type(&self) -> u32;
    fn features(&self) -> u32;
    fn pass_guest_memory(&mut self, _guest_memory: VirtioGuestMemoryHandle);
    fn tick(&mut self, queue: &mut VirtioQueue) -> bool;
}

pub struct VirtioGuestMemoryHandle{
    mem: GuestMemoryHandle,
}

impl VirtioGuestMemoryHandle {
    pub fn new(mem: GuestMemoryHandle) -> Self {
        Self{
            mem
        }
    }

    pub fn read_u16(&self, addr: u64) -> u16{
        const LENGTH: u64 = 2;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region.read((addr - mem_region.mem_offset) as usize, LENGTH as usize).unwrap();
                return u16::from_le_bytes([data[0], data[1]]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn read_u32(&self, addr: u64) -> u32{
        const LENGTH: u64 = 4;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region.read((addr - mem_region.mem_offset) as usize, LENGTH as usize).unwrap();
                return u32::from_le_bytes([data[0], data[1], data[2], data[3]]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn read_u64(&self, addr: u64) -> u64{
        const LENGTH: u64 = 8;

        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset + mem_region.mem_size as u64;
            if addr >= start && addr + LENGTH <= end {
                let data = mem_region.read((addr - mem_region.mem_offset) as usize, LENGTH as usize).unwrap();
                return u64::from_le_bytes([data[0], data[1], data[2], data[3], data[4], data[5], data[6], data[7]]);
            }
        }

        println!("Virtio read a addr outside of mapped scope!");
        0
    }

    pub fn write_u16(&mut self, addr: u64, val: u16){
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

    pub fn write_u32(&mut self, addr: u64, val: u32){
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

    pub fn write_guest_memory(&mut self, addr: u64, data: &[u8]){
        let borrow = self.mem.lock().unwrap();
        for mem_region in borrow.iter() {
            let start = mem_region.mem_offset;
            let end = mem_region.mem_offset as usize + mem_region.mem_size;
            if addr >= start && addr as usize + data.len() <= end {
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

    pub fn pop_avail(&mut self, mem: &VirtioGuestMemoryHandle) -> Option<u16> {
        let avail_idx = mem.read_u16(self.avail_addr + 2);
        
        if self.last_avail_idx == avail_idx {
            return None;
        }

        let ring_offset = 4 + (self.last_avail_idx % self.size) as u64 * 2;
        let head = mem.read_u16(self.avail_addr + ring_offset);

        self.last_avail_idx += 1;
        Some(head)
    }

    pub fn push_used(&self, mem: &mut VirtioGuestMemoryHandle, head: u16, len: u32) {
        let used_idx = mem.read_u16(self.used_addr + 2);

        let offset = 4 + (used_idx % self.size) as u64 * 8;

        mem.write_u32(self.used_addr + offset, head as u32);
        mem.write_u32(self.used_addr + offset + 4, len);

        mem.write_u16(self.used_addr + 2, used_idx + 1);
    }

    pub fn get_descriptor(&self, mem: &VirtioGuestMemoryHandle, index: u16) -> VirtqDesc {
        let desc_addr = self.desc_addr + (index as u64) * 16;

        VirtqDesc {
            addr:  mem.read_u64(desc_addr + 0),
            len:   mem.read_u32(desc_addr + 8),
            flags: mem.read_u16(desc_addr + 12),
            next:  mem.read_u16(desc_addr + 14),
        }
    }

    pub fn read_avail_entry(&self, mem: &VirtioGuestMemoryHandle, idx: u16) -> u16 {
        let ring_offset =
            4 + ((idx % self.size) as u64) * 2;

        mem.read_u16(self.avail_addr + ring_offset)
    }
}