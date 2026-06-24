use std::{
    fs::File,
    io::{Read, Write},
    os::fd::FromRawFd,
};

use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

const VIRTIO_NET_HDR_SIZE: usize = 12;

const MAC_ADDRESS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

pub struct VirtioNetConfig {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
}

impl VirtioNetConfig {
    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(length);
        buf.extend_from_slice(&self.mac);
        buf.extend_from_slice(&self.status.to_le_bytes());
        buf.extend_from_slice(&self.max_virtqueue_pairs.to_le_bytes());
        buf.extend_from_slice(&self.mtu.to_le_bytes());
        buf.resize(length, 0);
        buf
    }
}

pub struct NetVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    tap: File,
    config: VirtioNetConfig,
}

impl NetVirtio {
    pub fn new(tap_name: &str) -> Self {
        let tap = Self::open_tap(tap_name);
        Self {
            guest_memory: None,
            tap,
            config: VirtioNetConfig {
                mac: MAC_ADDRESS,
                status: 0,
                max_virtqueue_pairs: 1,
                mtu: 1500,
            },
        }
    }

    fn open_tap(name: &str) -> File {
        const TUNSETIFF: libc::c_ulong = 0x4004_54ca;
        const IFF_TAP: libc::c_short = 0x0002;
        const IFF_NO_PI: libc::c_short = 0x1000;

        let fd = unsafe {
            libc::open(
                b"/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK,
            )
        };
        assert!(fd >= 0, "failed to open /dev/net/tun");

        let mut ifr = [0u8; 40];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(libc::IFNAMSIZ as usize - 1);
        ifr[..len].copy_from_slice(&name_bytes[..len]);
        let flags = IFF_TAP | IFF_NO_PI;
        ifr[16..18].copy_from_slice(&flags.to_le_bytes());

        let ret = unsafe {
            libc::ioctl(fd, TUNSETIFF, &ifr as *const _ as *const libc::c_void)
        };
        assert!(
            ret >= 0,
            "TUNSETIFF failed for '{}' (errno: {}, run scripts/enable_tap.sh first, or use cargo-run.sh for cap_net_admin)",
            name,
            unsafe { *libc::__errno_location() },
        );

        unsafe { File::from_raw_fd(fd) }
    }

    fn process_tx(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };
        let mut did_work = false;

        while let Some(head) = queue.pop_avail(guest_memory) {
            let mut frame = Vec::new();
            let mut idx = head;
            let mut is_first = true;

            loop {
                let desc = queue.get_descriptor(guest_memory, idx);
                let mut buf = vec![0u8; desc.len as usize];
                guest_memory.read_guest_memory(desc.addr, &mut buf);

                if is_first {
                    if desc.len > VIRTIO_NET_HDR_SIZE as u32 {
                        frame.extend_from_slice(&buf[VIRTIO_NET_HDR_SIZE..]);
                    }
                    is_first = false;
                } else {
                    frame.extend_from_slice(&buf);
                }

                if desc.flags & 1 == 0 {
                    break;
                }
                idx = desc.next;
            }

            if !frame.is_empty() {
                let _ = self.tap.write_all(&frame);
            }

            queue.push_used(guest_memory, head, 0);
            did_work = true;
        }

        did_work
    }

    fn process_rx(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };
        let mut did_work = false;

        loop {
            let mut tap_buf = vec![0u8; 2048];
            let n = match self.tap.read(&mut tap_buf) {
                Ok(0) | Err(_) => break,
                Ok(n) => n,
            };

            let Some(head) = queue.pop_avail(guest_memory) else {
                break;
            };

            let desc = queue.get_descriptor(guest_memory, head);
            let write_len = if desc.flags & 2 != 0 {
                let total = VIRTIO_NET_HDR_SIZE + n;
                let max = desc.len as usize;
                let actual = total.min(max);
                let hdr = [0u8; VIRTIO_NET_HDR_SIZE];
                guest_memory.write_guest_memory(desc.addr, &hdr);
                guest_memory.write_guest_memory(
                    desc.addr + VIRTIO_NET_HDR_SIZE as u64,
                    &tap_buf[..n.min(max.saturating_sub(VIRTIO_NET_HDR_SIZE))],
                );
                actual as u32
            } else {
                0
            };

            queue.push_used(guest_memory, head, write_len);
            did_work = true;
        }

        did_work
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
            0 => self.process_rx(queue),
            1 => self.process_tx(queue),
            _ => false,
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        if !queues.is_empty() && queues[0].ready {
            self.process_rx(&mut queues[0])
        } else {
            false
        }
    }
}
