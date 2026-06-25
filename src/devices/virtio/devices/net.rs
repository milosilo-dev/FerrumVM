use std::{
    fs::File,
    io::{Read, Write},
    os::fd::FromRawFd,
};

use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

const VIRTIO_NET_HDR_SIZE: usize = 12;

const MAC_ADDRESS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];

const VIRTIO_NET_F_MAC: u32 = 1 << 5;
const VIRTIO_NET_F_STATUS: u32 = 1 << 16;

const VIRTIO_NET_S_LINK_UP: u16 = 1;

/// Holds a copy of all VirtioQueue fields so we can reconstruct a temporary
/// queue for piggyback processing.  Only the indices are ever written back
/// to the real queue (sync_indices_to) — the addresses belong to the MMIO
/// transport and must never be overwritten.
#[derive(Clone, Copy)]
struct VirtioQueueSnapshot {
    desc_addr: u64,
    avail_addr: u64,
    used_addr: u64,
    size: u16,
    ready: bool,
    last_avail_idx: u16,
    last_used_idx: u16,
}

impl VirtioQueueSnapshot {
    fn from(q: &VirtioQueue) -> Self {
        Self {
            desc_addr: q.desc_addr,
            avail_addr: q.avail_addr,
            used_addr: q.used_addr,
            size: q.size,
            ready: q.ready,
            last_avail_idx: q.last_avail_idx,
            last_used_idx: q.last_used_idx,
        }
    }

    /// Overwrite only the indices on `q` so that already-consumed entries are
    /// not processed again.  Do NOT touch the addresses – those are owned by
    /// the MMIO transport.
    fn sync_indices_to(&self, q: &mut VirtioQueue) {
        q.last_avail_idx = self.last_avail_idx;
        q.last_used_idx = self.last_used_idx;
    }

    /// Build a full VirtioQueue from the snapshot (addresses + indices) for
    /// use as a temporary when piggybacking RX on a TX kick.
    fn to_queue(&self) -> VirtioQueue {
        VirtioQueue {
            desc_addr: self.desc_addr,
            avail_addr: self.avail_addr,
            used_addr: self.used_addr,
            size: self.size,
            ready: self.ready,
            last_avail_idx: self.last_avail_idx,
            last_used_idx: self.last_used_idx,
        }
    }
}

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
    /// Per-queue snapshots so we can piggyback RX processing on TX kicks.
    snapshots: [VirtioQueueSnapshot; 2],
}

impl NetVirtio {
    pub fn new(tap_name: &str) -> Self {
        let tap = Self::open_tap(tap_name);
        Self {
            guest_memory: None,
            tap,
            config: VirtioNetConfig {
                mac: MAC_ADDRESS,
                status: VIRTIO_NET_S_LINK_UP,
                max_virtqueue_pairs: 1,
                mtu: 1500,
            },
            snapshots: [VirtioQueueSnapshot {
                desc_addr: 0,
                avail_addr: 0,
                used_addr: 0,
                size: 0,
                ready: false,
                last_avail_idx: 0,
                last_used_idx: 0,
            }; 2],
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
        VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        match queue_sel {
            1 => {
                // Save a full snapshot (addresses + indices) of the RX queue
                // so it can be reconstructed for piggyback processing later.
                self.snapshots[0] = VirtioQueueSnapshot::from(queue);

                let did_work = self.process_rx(queue);

                // Persist the updated indices back into the snapshot.
                self.snapshots[0].last_avail_idx = queue.last_avail_idx;
                self.snapshots[0].last_used_idx = queue.last_used_idx;

                did_work
            }
            0 => {
                // Sync our TX indices so we don't re-process old entries.
                self.snapshots[1].sync_indices_to(queue);

                let tx_work = self.process_tx(queue);

                self.snapshots[1].last_avail_idx = queue.last_avail_idx;
                self.snapshots[1].last_used_idx = queue.last_used_idx;

                // Piggyback: process RX immediately so the guest gets
                // responses (e.g. ARP replies, DHCP offers) without
                // waiting for the next periodic tick().
                let rx_work = if self.snapshots[0].ready {
                    let mut rx = self.snapshots[0].to_queue();
                    let did_rx = self.process_rx(&mut rx);
                    self.snapshots[0].last_avail_idx = rx.last_avail_idx;
                    self.snapshots[0].last_used_idx = rx.last_used_idx;
                    did_rx
                } else {
                    false
                };

                tx_work || rx_work
            }
            _ => false,
        }
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        let mut did_work = false;

        if queues.len() > 0 && queues[0].ready {
            self.snapshots[0].sync_indices_to(&mut queues[0]);
            if self.process_rx(&mut queues[0]) {
                did_work = true;
            }
            self.snapshots[0].last_avail_idx = queues[0].last_avail_idx;
            self.snapshots[0].last_used_idx = queues[0].last_used_idx;
        }

        if queues.len() > 1 && queues[1].ready {
            self.snapshots[1].sync_indices_to(&mut queues[1]);
            if self.process_tx(&mut queues[1]) {
                did_work = true;
            }
            self.snapshots[1].last_avail_idx = queues[1].last_avail_idx;
            self.snapshots[1].last_used_idx = queues[1].last_used_idx;
        }

        did_work
    }
}
