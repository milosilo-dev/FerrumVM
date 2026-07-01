use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

pub struct ConsoleVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
}

impl ConsoleVirtio {
    pub fn new() -> Self {
        ConsoleVirtio {
            guest_memory: None,
        }
    }

    // Send Packet to client
    fn tick_rx_queue(&mut self, _queue: &mut VirtioQueue) -> bool {
        true
    }

    // Handle packet from client
    fn tick_tx_queue(&mut self, _queue: &mut VirtioQueue) -> bool {
        // Output the data on the virt queue
        true
    }
}

impl VirtioDevice for ConsoleVirtio {
    fn virtio_type(&self) -> u32 {
        0x3
    }

    fn features(&self) -> u64 {
        0x0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        match queue_sel {
            0 => self.tick_rx_queue(queue),
            1 => self.tick_tx_queue(queue),
            _ => false,
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        vec![0; length]
    }

    fn update(&mut self, _queues: &mut [crate::devices::virtio::virtio::VirtioQueue]) -> bool {
        false
    }
}