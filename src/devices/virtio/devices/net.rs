use crate::{
    devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue},
    host::tap::create_tap,
};
use std::fs::File;
use std::io::{Read, Write};

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

struct VirtioNetConfig {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
    speed: u32,
    duplex: u8,
    rss_max_key_size: u8,
    rss_max_indirection_table_length: u16,
    supported_hash_types: u32,
}

impl VirtioNetConfig {
    fn new(
        mac: [u8; 6],
        status: u16,
        max_virtqueue_pairs: u16,
        mtu: u16,
        speed: u32,
        duplex: u8,
        rss_max_key_size: u8,
        rss_max_indirection_table_length: u16,
        supported_hash_types: u32,
    ) -> Self {
        Self {
            mac,
            status,
            max_virtqueue_pairs,
            mtu,
            speed,
            duplex,
            rss_max_key_size,
            rss_max_indirection_table_length,
            supported_hash_types,
        }
    }

    fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut bytes = self.mac.to_vec();
        bytes.extend(self.status.to_le_bytes());
        bytes.extend(self.max_virtqueue_pairs.to_le_bytes());
        bytes.extend(self.mtu.to_le_bytes());
        bytes.extend(self.speed.to_le_bytes());
        bytes.extend(self.duplex.to_le_bytes());
        bytes.extend(self.rss_max_key_size.to_le_bytes());
        bytes.extend(self.rss_max_indirection_table_length.to_le_bytes());
        bytes.extend(self.supported_hash_types.to_le_bytes());
        bytes.resize(length, 0);
        bytes
    }
}

pub struct NetVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioNetConfig,
    tap: File,
}

impl NetVirtio {
    pub fn new() -> Self {
        let tap = create_tap("tap0");

        Self {
            guest_memory: None,
            config: VirtioNetConfig::new(
                [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
            tap,
        }
    }

    fn tick_rx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut work_done = false;
        let mut frame_buf = [0u8; 65536];

        loop {
            // Read from TAP first — non-blocking; only pop a descriptor when data is available
            let n = match self.tap.read(&mut frame_buf) {
                Ok(0) => break,
                Ok(n) => n,
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
                Err(_) => return work_done,
            };

            let Some(head) = queue.pop_avail(guest_memory) else {
                break;
            };

            let desc = queue.get_descriptor(guest_memory, head);

            // Walk descriptor chain and find first writable buffer
            let mut write_desc = desc;
            while write_desc.flags & VIRTQ_DESC_F_WRITE == 0
                && (write_desc.flags & VIRTQ_DESC_F_NEXT) != 0
            {
                write_desc = queue.get_descriptor(guest_memory, write_desc.next);
            }

            // Prepend 12-byte virtio_net_hdr_v1 (all zeros = no offload)
            const HDR_LEN: usize = 12;
            let total_len = (HDR_LEN + n).min(write_desc.len as usize);
            let payload_len = total_len.saturating_sub(HDR_LEN);

            let mut out = vec![0u8; HDR_LEN];
            out.extend_from_slice(&frame_buf[..payload_len]);

            guest_memory.write_guest_memory(write_desc.addr, &out);
            queue.push_used(guest_memory, head, total_len as u32);
            work_done = true;
        }

        work_done
    }

    // Handle packet from client
    fn tick_tx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut work_done = false;

        while let Some(head) = queue.pop_avail(guest_memory) {
            let mut desc = queue.get_descriptor(guest_memory, head);

            // Walk full descriptor chain and collect READABLE buffers only
            let mut packet_buf = Vec::new();

            loop {
                // Only read descriptors that are NOT write-only
                if (desc.flags & VIRTQ_DESC_F_WRITE) == 0 {
                    let mut buf = vec![0u8; desc.len as usize];
                    guest_memory.read_guest_memory(desc.addr, &mut buf);
                    packet_buf.extend_from_slice(&buf);
                }

                if (desc.flags & VIRTQ_DESC_F_NEXT) == 0 {
                    break;
                }

                desc = queue.get_descriptor(guest_memory, desc.next);
            }

            eprintln!("netio-tx: packet len={}", packet_buf.len());

            if let Err(e) = self.tap.write_all(&packet_buf) {
                eprintln!("netio-tx: tap write failed: {:?}", e);
                continue; // DO NOT complete incorrectly
            }

            // Correct completion: entire packet consumed
            queue.push_used(guest_memory, head, packet_buf.len() as u32);
            work_done = true;
        }

        work_done
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u64 {
        1 << 5
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        match queue_sel {
            0 => self.tick_rx_queue(queue),
            1 => self.tick_tx_queue(queue),
            _ => true,
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        let queue = &mut queues[0]; // RX queue stored inside the device
        self.tick_rx_queue(queue) // || self.tick_tx_queue(queue)
    }
}
