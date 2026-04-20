use std::{cell::RefCell, ptr, rc::Rc};

pub struct MemoryRegion {
    ptr: *mut u8,
    pub mem_size: usize,
    pub mem_offset: u64,
}

pub type GuestMemoryHandle = Rc<RefCell<Vec<MemoryRegion>>>;

impl MemoryRegion {
    pub fn new(ptr: *mut u8, mem_size: usize, mem_offset: u64) -> Self{
        Self {
            ptr,
            mem_size: mem_size,
            mem_offset: mem_offset
        }
    }

    // Addr is an offset from the start of the memory region
    pub fn write(&self, data: &mut [u8], addr: usize) {
        if self.ptr.is_null() || addr + data.len() > self.mem_size {
            return;
        }

        unsafe {
            ptr::copy_nonoverlapping(
                data.as_ptr(),
                self.ptr
                    .add(addr),
                data.len(),
            );
        }
    }

    pub fn read(&mut self, addr: usize, length: usize) -> Option<Vec<u8>> {
        if self.ptr.is_null() || addr + length > self.mem_size {
            return None;
        }

        unsafe {
            let start_ptr = self.ptr.add(addr);
            Some(std::slice::from_raw_parts_mut(start_ptr, length).to_vec())
        }
    }
}
