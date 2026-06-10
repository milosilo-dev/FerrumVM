use crate::devices::virtio::{devices::blk::BlkRequest, virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue}};

struct VirtioNetConfig {

}

impl VirtioNetConfig {
    fn new() -> Self{
        Self{}
    }

    fn to_bytes(&self, length: usize) -> Vec<u8> {
        vec![0; length]
    }
}

struct PacketDesc{
    flags: u8,                  // Bit 0: Needs checksum; Bit 1: Received packet has valid data; Bit 2: If VIRTIO_NET_F_RSC_EXT was negotiated
    segmentation_offload: u8,   // 0:None 1:TCPv4 3:UDP 4:TCPv6 0x80:ECN
    desc_length: u16,         // Size of desc to be used during segmentation.
    segment_length: u16,        // Maximum segment size (not including desc).
    checksum_start: u16,        // The position to begin calculating the checksum.
    checksum_offset: u16,       // The position after ChecksumStart to store the checksum.
    buffer_count: u16,          // Used when merging buffers.
}

impl PacketDesc{
    fn new(base_ptr: u64, guest_memory: &VirtioGuestMemoryHandle) -> Self{
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
}

pub struct NetVirtio{
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioNetConfig,
}

impl NetVirtio{
    pub fn new() -> Self{
        Self { 
            guest_memory: None, 
            config: VirtioNetConfig::new() 
        }
    }

    // Send Packet to client
    fn tick_rx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        while let Some(head) = queue.pop_avail(guest_memory) {
            let desc = queue.get_descriptor(guest_memory, head);

            if desc.flags & 1 != 0 {
                // Has seperate descriptor for packet data
                todo!();
            }

            // write virtio_net_hdr
            guest_memory.write_guest_memory(desc.addr, &header_bytes);

            // write packet after header
            guest_memory.write_guest_memory(desc.addr + 12, ethernet_frame);

            queue.push_used(
                guest_memory,
                head,
                12 + ethernet_frame.len() as u32
            );
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
            let header = PacketDesc::new(desc.addr, guest_memory);

            let packet_addr = if desc.flags & 1 != 0 {
                // Has seperate descriptor for packet data
                let data_desc = queue.get_descriptor(&guest_memory, desc.next);
                data_desc.addr
            } else {
                desc.addr + 12
            }

            let mut packet = vec![0u8; desc.len as usize];
            guest_memory.read_guest_memory(packet_addr, &mut packet);

            queue.push_used(guest_memory, head, desc.len);
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
        match queue_sel{
            0 => {
                self.tick_rx_queue(queue)
            }
            1 => {
                self.tick_tx_queue(queue)
            }
            _ => true
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }
}