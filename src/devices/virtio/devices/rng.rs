use vmm_sys_util::rand;

use crate::{devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue}};

pub struct RngVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
}

impl RngVirtio {
    pub fn new() -> Self{
        Self{
            guest_memory: None
        }
    }

    pub fn get_random(length: usize) -> Vec<u8>{
        rand::rand_bytes(length)
    }
}

impl VirtioDevice for RngVirtio {
    fn virtio_type(&self) -> u32 {
        0x04
    }

    fn features(&self) -> u32 {
        0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue: &mut VirtioQueue) -> bool {
        if self.guest_memory.is_none(){
            return false;
        }

        let mut guest_memory = self.guest_memory.as_mut().unwrap();
        let mut did_work = false;

        while let Some(head) = queue.pop_avail(&guest_memory) {
            let desc = queue.get_descriptor(&guest_memory, head);

            if desc.flags & 2 == 0 {
                panic!("virtio-rng got non-writable buffer");
            }

            let data = Self::get_random(desc.len as usize);
            guest_memory.write_guest_memory(desc.addr, &data);

            queue.push_used(&mut guest_memory, head, desc.len);

            did_work = true;
        }

        did_work
    }
}