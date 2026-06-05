use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

pub struct CntVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
}

impl CntVirtio {
    pub fn new() -> Self {
        Self { guest_memory: None }
    }
}

impl VirtioDevice for CntVirtio {
    fn virtio_type(&self) -> u32 {
        0x10
    }

    fn features(&self) -> u32 {
        0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue: &mut VirtioQueue) -> bool {
        if self.guest_memory.is_none() {
            return false;
        }

        let mut guest_memory = self.guest_memory.as_mut().unwrap();
        let mut did_work = false;

        while let Some(head) = queue.pop_avail(&guest_memory) {
            let desc = queue.get_descriptor(&guest_memory, head);

            if desc.flags & 2 == 0 || desc.flags & 1 == 0 {
                panic!("virtio-cnt got unusable buffer: {:b}", desc.flags);
            }

            let desc2 = queue.get_descriptor(&guest_memory, desc.next);

            let input = guest_memory.read_u32(desc.addr);
            let output = input + 1;
            guest_memory.write_u32(desc2.addr, output);

            queue.push_used(&mut guest_memory, head, desc.len);

            did_work = true;
        }

        did_work
    }

    fn read_config(&self, _length: usize) -> Vec<u8> {
        vec![]
    }
}
