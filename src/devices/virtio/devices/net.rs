use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

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

struct PacketDesc {
    flags: u8, // Bit 0: Needs checksum; Bit 1: Received packet has valid data; Bit 2: If VIRTIO_NET_F_RSC_EXT was negotiated
    segmentation_offload: u8, // 0:None 1:TCPv4 3:UDP 4:TCPv6 0x80:ECN
    desc_length: u16, // Size of desc to be used during segmentation.
    segment_length: u16, // Maximum segment size (not including desc).
    checksum_start: u16, // The position to begin calculating the checksum.
    checksum_offset: u16, // The position after ChecksumStart to store the checksum.
    buffer_count: u16, // Used when merging buffers.
}

impl PacketDesc {
    fn new(
        flags: u8,
        segmentation_offload: u8,
        desc_length: u16,
        segment_length: u16,
        buffer_count: u16,
        checksum_offset: u16,
        checksum_start: u16,
    ) -> Self {
        Self {
            flags,
            segmentation_offload,
            desc_length,
            segment_length,
            buffer_count,
            checksum_start,
            checksum_offset,
        }
    }

    fn from_guest_mem(base_ptr: u64, guest_memory: &VirtioGuestMemoryHandle) -> Self {
        Self {
            flags: guest_memory.read_byte(base_ptr),
            segmentation_offload: guest_memory.read_byte(base_ptr + 1),
            desc_length: guest_memory.read_u16(base_ptr + 2),
            segment_length: guest_memory.read_u16(base_ptr + 4),
            checksum_start: guest_memory.read_u16(base_ptr + 6),
            checksum_offset: guest_memory.read_u16(base_ptr + 8),
            buffer_count: guest_memory.read_u16(base_ptr + 10),
        }
    }

    fn to_bytes(&self) -> Vec<u8> {
        let mut bytes = vec![self.flags];
        bytes.push(self.segmentation_offload);
        bytes.extend(self.desc_length.to_le_bytes());
        bytes.extend(self.segment_length.to_le_bytes());
        bytes.extend(self.checksum_start.to_le_bytes());
        bytes.extend(self.checksum_offset.to_le_bytes());
        bytes.extend(self.buffer_count.to_le_bytes());
        bytes
    }
}

pub struct NetVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioNetConfig,
}

impl NetVirtio {
    pub fn new() -> Self {
        Self {
            guest_memory: None,
            config: VirtioNetConfig::new([0, 0, 0, 0, 0, 0], 0, 0, 0, 0, 0, 0, 0, 0),
        }
    }

    // Send Packet to client
    fn tick_rx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        while let Some(head) = queue.pop_avail(guest_memory) {
            let desc = queue.get_descriptor(guest_memory, head);

            let packet_desc = if desc.flags & 1 != 0 {
                // Has seperate descriptor for packet data
                queue.get_descriptor(guest_memory, desc.next)
            } else {
                desc
            };

            // Use these to create the packet
            let header = PacketDesc::new(0, 0, 0, 0, 0, 0, 0);
            let ethernet_frame: [u8; 10] = [0; 10];

            guest_memory.write_guest_memory(packet_desc.addr, header.to_bytes().as_slice());
            guest_memory.write_guest_memory(packet_desc.addr + 12, &ethernet_frame);

            queue.push_used(guest_memory, head, 12 + ethernet_frame.len() as u32);
        }
        true
    }

    // Handle packet from client
    fn tick_tx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        while let Some(head) = queue.pop_avail(guest_memory) {
            let desc = queue.get_descriptor(&guest_memory, head);
            let header = PacketDesc::from_guest_mem(desc.addr, guest_memory);

            let packet_desc = if desc.flags & 1 != 0 {
                // Has seperate descriptor for packet data
                queue.get_descriptor(&guest_memory, desc.next)
            } else {
                desc
            };

            let mut packet = vec![0u8; packet_desc.len as usize];
            guest_memory.read_guest_memory(packet_desc.addr, &mut packet);

            // Use packet data

            queue.push_used(guest_memory, head, packet_desc.len);
        }
        true
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u32 {
        0
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
}
