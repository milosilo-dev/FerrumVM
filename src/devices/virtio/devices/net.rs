use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::process::Command;

const RX_QUEUE_FILL: usize = 32;

#[repr(C)]
#[derive(Clone, Copy)]
struct VirtioNetHdr {
    flags: u8,
    gso_type: u8,
    hdr_len: u16,
    gso_size: u16,
    csum_start: u16,
    csum_offset: u16,
    num_buffers: u16,
}

impl VirtioNetHdr {
    fn new() -> Self {
        Self {
            flags: 0,
            gso_type: 0,
            hdr_len: 0,
            gso_size: 0,
            csum_start: 0,
            csum_offset: 0,
            num_buffers: 0,
        }
    }

    fn as_bytes(&self) -> [u8; 12] {
        let mut out = [0u8; 12];
        out[0] = self.flags;
        out[1] = self.gso_type;
        out[2..4].copy_from_slice(&self.hdr_len.to_le_bytes());
        out[4..6].copy_from_slice(&self.gso_size.to_le_bytes());
        out[6..8].copy_from_slice(&self.csum_start.to_le_bytes());
        out[8..10].copy_from_slice(&self.csum_offset.to_le_bytes());
        out[10..12].copy_from_slice(&self.num_buffers.to_le_bytes());
        out
    }
}

pub struct NetVirtio {
    mem: Option<VirtioGuestMemoryHandle>,
    tap: std::fs::File,

    // IMPORTANT: track link state explicitly
    link_up: bool,
}

impl NetVirtio {
    pub fn new() -> Self {
        let tap = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .expect("failed to open /dev/net/tun");

        let _ = Command::new("target/debug/nethelper").status();

        Self {
            mem: None,
            tap,
            link_up: true,
        }
    }

    fn rx(&mut self, q: &mut VirtioQueue) -> bool {
        let Some(mem) = self.mem.as_mut() else {
            return false;
        };

        let mut did = false;
        let mut buf = [0u8; 2048];

        while let Some(head) = q.pop_avail(mem) {
            let desc = q.get_descriptor(mem, head);

            let n = match self.tap.read(&mut buf) {
                Ok(n) => n,
                Err(_) => break,
            };

            let hdr = VirtioNetHdr::new();
            let hdr_bytes = hdr.as_bytes();

            if (desc.len as usize) < n + hdr_bytes.len() {
                continue;
            }

            mem.write_guest_memory(desc.addr, &hdr_bytes);
            mem.write_guest_memory(desc.addr + hdr_bytes.len() as u64, &buf[..n]);

            q.push_used(mem, head, (hdr_bytes.len() + n) as u32);
            did = true;
        }

        did
    }

    fn tx(&mut self, q: &mut VirtioQueue) -> bool {
        let Some(mem) = self.mem.as_mut() else {
            return false;
        };

        let mut did = false;

        while let Some(head) = q.pop_avail(mem) {
            let mut desc = q.get_descriptor(mem, head);

            let mut packet = Vec::new();

            loop {
                let mut buf = vec![0u8; desc.len as usize];
                mem.read_guest_memory(desc.addr, &mut buf);
                packet.extend_from_slice(&buf);

                if desc.flags & 1 == 0 {
                    break;
                }

                desc = q.get_descriptor(mem, desc.next);
            }

            let _ = self.tap.write_all(&packet);
            q.push_used(mem, head, packet.len() as u32);

            did = true;
        }

        did
    }

    // -------------------------
    // CRITICAL FIX:
    // Pre-fill RX buffers like Linux expects
    // -------------------------
    fn fill_rx_queue(&mut self, q: &mut VirtioQueue) {
        let Some(mem) = self.mem.as_mut() else { return };

        for _ in 0..RX_QUEUE_FILL {
            if let Some(head) = q.pop_avail(mem) {
                // immediately return descriptor to used ring
                q.push_used(mem, head, 0);
            }
        }
    }

    fn link_status(&self) -> u16 {
        if self.link_up { 1 } else { 0 }
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        1
    }

    fn features(&self) -> u32 {
        (1 << 0)  // CSUM
        | (1 << 5)  // MAC
        | (1 << 16) // STATUS
    }

    fn pass_guest_memory(&mut self, mem: VirtioGuestMemoryHandle) {
        self.mem = Some(mem);
    }

    fn tick(&mut self, q: usize, queues: &mut VirtioQueue) -> bool {
        match q {
            0 => {
                self.fill_rx_queue(queues); // 🔥 IMPORTANT for Linux init
                self.rx(queues)
            }
            1 => self.tx(queues),
            _ => false,
        }
    }

    fn read_config(&self, _len: usize) -> Vec<u8> {
        // minimal config: MAC + status
        let mut cfg = vec![0u8; 18];

        cfg[0..6].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        cfg[16..18].copy_from_slice(&self.link_status().to_le_bytes());

        cfg
    }

    fn write_config(&mut self, _offset: usize, _data: &[u8]) {}

    fn update(&mut self, _queues: &mut [VirtioQueue]) -> bool {
        false
    }
}
