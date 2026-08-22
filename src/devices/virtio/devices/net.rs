use std::collections::VecDeque;

use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle};

const VIRTIO_NET_F_MAC: u8 = 5;
const VIRTIO_NET_F_STATUS: u8 = 16;

struct NetVirtioConfig {
    mac: [u8; 6],
    status: u16,
}

impl NetVirtioConfig {
    pub fn new(
        mac: [u8; 6],
        status: u16,
    ) -> Self {
        return Self {
            mac,
            status,
        };
    }

    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = self.mac.to_vec();
        buf.extend(self.status.to_le_bytes());

        buf.resize(length, 0);

        buf
    }
}

pub struct NetVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    packet_recive_queue: VecDeque<Vec<u8>>,
    config: NetVirtioConfig,
}

impl NetVirtio {
    pub fn new() -> Self {
        let mut ret = Self { guest_memory: None, packet_recive_queue: VecDeque::new(), config: NetVirtioConfig::new([0x02, 0x00, 0x00, 0x00, 0x00, 0x01], 0) };
        ret.packet_recive_queue.push_back(vec![0x67, 0x67, 0x54, 0x69]);
        ret
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u64 {
        1 << VIRTIO_NET_F_MAC | 1 << VIRTIO_NET_F_STATUS
    }

    fn pass_guest_memory(
        &mut self,
        guest_memory: crate::devices::virtio::virtio::VirtioGuestMemoryHandle,
    ) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(
        &mut self,
        queue_sel: usize,
        queue: &mut crate::devices::virtio::virtio::VirtioQueue,
    ) -> bool {
        match queue_sel {
            0 => {
                // Recive queue, Where we will send packets into the guest
                let Some(guest_memory) = self.guest_memory.as_mut() else {
                    return false;
                };

                let mut did_work: bool = false;
                while let Some(eth_frame) = self.packet_recive_queue.pop_front(){
                    let Some(head) = queue.pop_avail(guest_memory) else {
                        self.packet_recive_queue.push_front(eth_frame);
                        return did_work;
                    };

                    let desc = queue.get_descriptor(guest_memory, head);
                    let hdr = [0u8, 0, 0, 0, 0, 0, 0, 0, 0, 0];

                    if eth_frame.len() + hdr.len() > desc.len as usize {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }

                    guest_memory.write_guest_memory(desc.addr, &hdr);
                    guest_memory.write_guest_memory(desc.addr + hdr.len() as u64, &eth_frame);
                    queue.push_used(guest_memory, head, (hdr.len() + eth_frame.len()) as u32);

                    did_work = true;
                }
                return did_work;
            }
            1 => {
                // Transmit queue, Where we will read pakets from the guest
                let Some(guest_memory) = self.guest_memory.as_mut() else {
                    return false;
                };

                let mut did_work: bool = false;
                while let Some(head) = queue.pop_avail(guest_memory) {
                    let desc = queue.get_descriptor(guest_memory, head);

                    let hdr_size = 10u32;
                    if desc.len < hdr_size {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }

                    let mut eth_frame: Vec<u8> = vec![0; (desc.len - hdr_size) as usize];
                    guest_memory.read_guest_memory(desc.addr + hdr_size as u64, &mut eth_frame);
                    print!("Queue sent: {:X?}\r\n", eth_frame);
        
                    queue.push_used(guest_memory, head, desc.len);
                    did_work = true;
                }
                return did_work;
            }
            _ => {false}
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, _queues: &mut [crate::devices::virtio::virtio::VirtioQueue]) -> bool { false }
}
