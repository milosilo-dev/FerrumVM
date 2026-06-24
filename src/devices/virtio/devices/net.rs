use crate::devices::virtio::virtio::{
    VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue,
};
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::fd::AsRawFd;
use std::mem;
use libc;

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
    irq_pending: bool,
}

impl NetVirtio {
    pub fn new() -> Self {
        let tap = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/net/tun")
            .expect("open /dev/net/tun failed");

        unsafe {
            let mut ifr: libc::ifreq = mem::zeroed();

            let name = b"ferrum-tap0\0";
            for (i, b) in name.iter().enumerate() {
                ifr.ifr_name[i] = *b as i8;
            }

            ifr.ifr_ifru.ifru_flags = (libc::IFF_TAP | libc::IFF_NO_PI) as i16;

            let res = libc::ioctl(tap.as_raw_fd(), libc::TUNSETIFF, &ifr);
            if res < 0 {
                panic!("TUNSETIFF failed: {}", std::io::Error::last_os_error());
            }

            // set non-blocking so rx() never blocks the VMM thread
            let flags = libc::fcntl(tap.as_raw_fd(), libc::F_GETFL, 0);
            libc::fcntl(tap.as_raw_fd(), libc::F_SETFL, flags | libc::O_NONBLOCK);
        }

        Self {
            mem: None,
            tap,
            irq_pending: false,
        }
    }

    // -------------------------
    // RX: TAP → GUEST
    // -------------------------
    fn rx(&mut self, q: &mut VirtioQueue) -> bool {
        let Some(mem) = self.mem.as_mut() else { return false };

        // only consume descriptors when data is available
        let mut pollfd = libc::pollfd {
            fd: self.tap.as_raw_fd(),
            events: libc::POLLIN,
            revents: 0,
        };
        let ret = unsafe { libc::poll(&mut pollfd, 1, 0) };
        if ret <= 0 || pollfd.revents & libc::POLLIN == 0 {
            return false;
        }

        let mut did = false;
        let mut buf = [0u8; 2048];

        while let Some(head) = q.pop_avail(mem) {
            let n = match self.tap.read(&mut buf) {
                Ok(n) => n,
                Err(_) => break,
            };

            let hdr = VirtioNetHdr::new();
            let hdr_bytes = hdr.as_bytes();
            let total = (hdr_bytes.len() + n) as u32;

            let desc = q.get_descriptor(mem, head);

            if (desc.len as usize) < hdr_bytes.len() + n {
                continue;
            }

            mem.write_guest_memory(desc.addr, &hdr_bytes);
            mem.write_guest_memory(desc.addr + hdr_bytes.len() as u64, &buf[..n]);

            q.push_used(mem, head, total);
            did = true;
        }

        if did {
            self.irq_pending = true;
        }
        did
    }

    // -------------------------
    // TX: GUEST → TAP
    // -------------------------
    fn tx(&mut self, q: &mut VirtioQueue) -> bool {
        let Some(mem) = self.mem.as_mut() else { return false };
        let mut did = false;

        while let Some(head) = q.pop_avail(mem) {
            let mut desc = q.get_descriptor(mem, head);
            let mut packet = Vec::new();
            let mut total_len = 0u32;

            loop {
                let mut buf = vec![0u8; desc.len as usize];
                mem.read_guest_memory(desc.addr, &mut buf);
                packet.extend_from_slice(&buf);
                total_len += desc.len;

                if desc.flags & 1 == 0 {
                    break;
                }
                desc = q.get_descriptor(mem, desc.next);
            }

            // strip 12-byte virtio-net header, send ethernet frame to TAP
            let payload = if packet.len() > 12 {
                &packet[12..]
            } else {
                &[]
            };
            let _ = self.tap.write_all(payload);

            // report total descriptor chain length, NOT stripped payload size
            q.push_used(mem, head, total_len);

            did = true;
        }

        if did {
            self.irq_pending = true;
        }
        did
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        1
    }

    fn features(&self) -> u32 {
        (1 << 5)  // MAC
    }

    fn pass_guest_memory(&mut self, mem: VirtioGuestMemoryHandle) {
        self.mem = Some(mem);
    }

    fn tick(&mut self, q: usize, queues: &mut VirtioQueue) -> bool {
        match q {
            0 => self.rx(queues),
            1 => self.tx(queues),
            _ => false,
        }
    }

    fn read_config(&self, _len: usize) -> Vec<u8> {
        let mut cfg = vec![0u8; 18];
        cfg[0..6].copy_from_slice(&[0x52, 0x54, 0x00, 0x12, 0x34, 0x56]);
        cfg
    }

    fn write_config(&mut self, _offset: usize, _data: &[u8]) {}

    fn update(&mut self, _queues: &mut [VirtioQueue]) -> bool {
        false
    }
}