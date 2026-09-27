use std::{
    ptr,
    sync::{Arc, Mutex},
};

pub struct MemoryRegion {
    pub ptr: *mut u8,
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub type GuestMemoryHandle = Arc<Mutex<Vec<MemoryRegion>>>;

unsafe impl Send for MemoryRegion {}

impl MemoryRegion {
    pub fn new(ptr: *mut u8, mem_size: usize, mem_offset: u64) -> Self {
        Self {
            ptr,
            mem_size,
            mem_offset,
        }
    }

    fn in_bounds(&self, addr: usize, length: usize) -> bool {
        !self.ptr.is_null() && addr <= self.mem_size && length <= self.mem_size - addr
    }

    pub fn write(&self, data: &[u8], addr: usize) {
        if !self.in_bounds(addr, data.len()) {
            return;
        }

        unsafe {
            ptr::copy_nonoverlapping(data.as_ptr(), self.ptr.add(addr), data.len());
        }
    }

    pub fn read(&self, addr: usize, length: usize) -> Option<Vec<u8>> {
        if !self.in_bounds(addr, length) {
            return None;
        }

        unsafe {
            let start_ptr = self.ptr.add(addr);
            Some(std::slice::from_raw_parts(start_ptr, length).to_vec())
        }
    }
}
