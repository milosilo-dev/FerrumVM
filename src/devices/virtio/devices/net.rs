use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};
use std::ffi::CString;
use std::fs::{File, OpenOptions};
use std::io::{Read, Write};
use std::os::unix::fs::OpenOptionsExt;
use std::os::unix::io::AsRawFd;

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
    flags: u8,
    segmentation_offload: u8,
    desc_length: u16,
    segment_length: u16,
    checksum_start: u16,
    checksum_offset: u16,
    buffer_count: u16,
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

    fn _from_guest_mem(base_ptr: u64, guest_memory: &VirtioGuestMemoryHandle) -> Self {
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
    tap: File,
}

impl NetVirtio {
    pub fn new() -> Self {
        let fd = OpenOptions::new()
            .read(true)
            .write(true)
            .custom_flags(libc::O_NONBLOCK)
            .open("/dev/net/tun")
            .expect("Failed to open /dev/net/tun");

        let mut ifr = [0u8; 64];
        let cname = CString::new("ferrum-tap0").unwrap();
        let name_bytes = cname.as_bytes_with_nul();
        ifr[..name_bytes.len()].copy_from_slice(name_bytes);
        let flags = (libc::IFF_TAP | libc::IFF_NO_PI) as u16;
        ifr[16..18].copy_from_slice(&flags.to_le_bytes());

        let ret = unsafe {
            libc::ioctl(
                fd.as_raw_fd(),
                libc::TUNSETIFF,
                &ifr as *const _ as *const libc::c_void,
            )
        };
        if ret < 0 {
            eprint!("TUNSETIFF failed: {}\n", std::io::Error::last_os_error());
        }

        let req = libc::ifreq {
            ifr_name: {
                let mut name = [0i8; libc::IFNAMSIZ];
                for (i, &b) in name_bytes.iter().enumerate() {
                    name[i] = b as i8;
                }
                name
            },
            ifr_ifru: libc::__c_anonymous_ifr_ifru {
                ifru_flags: (libc::IFF_UP | libc::IFF_RUNNING) as i16,
            },
        };
        unsafe {
            libc::ioctl(
                libc::socket(libc::AF_INET, libc::SOCK_DGRAM, 0),
                libc::SIOCSIFFLAGS,
                &req as *const _ as *const libc::c_void,
            );
        }

        Self {
            guest_memory: None,
            config: VirtioNetConfig::new(
                [0x52, 0x54, 0x00, 0x12, 0x34, 0x56],
                1,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ),
            tap: fd,
        }
    }

    fn tick_rx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut did_work = false;
        while let Some(head) = queue.pop_avail(guest_memory) {
            let desc = queue.get_descriptor(guest_memory, head);

            let packet_desc = if desc.flags & 1 != 0 {
                queue.get_descriptor(guest_memory, desc.next)
            } else {
                desc
            };

            let header = PacketDesc::new(0, 0, 0, 0, 1, 0, 0);

            let mut frame = vec![0u8; packet_desc.len as usize - 12];
            let n = match self.tap.read(&mut frame) {
                Ok(n) => {
                    did_work = true;
                    n
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    queue.last_avail_idx = queue.last_avail_idx.wrapping_sub(1);
                    break;
                }
                Err(_) => return false,
            };
            frame.truncate(n);

            guest_memory.write_guest_memory(packet_desc.addr, &header.to_bytes());
            guest_memory.write_guest_memory(packet_desc.addr + 12, &frame);

            queue.push_used(guest_memory, head, 12 + frame.len() as u32);
        }
        did_work
    }

    fn tick_tx_queue(&mut self, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut did_work = false;
        while let Some(head) = queue.pop_avail(guest_memory) {
            did_work = true;
            eprintln!("[dbg] TX pop head={} used_addr=0x{:x}", head, queue.used_addr);
            let desc = queue.get_descriptor(&guest_memory, head);

            let mut packet = Vec::new();
            let mut total_len = 0u32;

            if desc.flags & 1 != 0 {
                let mut current = desc.next;
                let mut max_chain = 0u16;
                loop {
                    if max_chain > 32 {
                        break;
                    }
                    max_chain += 1;
                    let data_desc = queue.get_descriptor(&guest_memory, current);
                    let mut buf = vec![0u8; data_desc.len as usize];
                    guest_memory.read_guest_memory(data_desc.addr, &mut buf);
                    packet.extend_from_slice(&buf);
                    total_len += data_desc.len;
                    if data_desc.flags & 1 == 0 {
                        break;
                    }
                    current = data_desc.next;
                }
            } else {
                if desc.len < 12 {
                    let _ = self.tap.write_all(&[]);
                    queue.push_used(guest_memory, head, desc.len);
                    continue;
                }
                let frame_len = desc.len as usize - 12;
                packet.resize(frame_len, 0);
                guest_memory.read_guest_memory(desc.addr + 12, &mut packet);
                total_len = desc.len;
            };

            let _ = self.tap.write_all(&packet);
            queue.push_used(guest_memory, head, total_len);
        }
        did_work
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u32 {
        (1 << 0)  // VIRTIO_NET_F_CSUM
        | (1 << 5)  // VIRTIO_NET_F_MAC
        | (1 << 16) // VIRTIO_NET_F_STATUS
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
        self.config.to_bytes(length)
    }

    fn write_config(&mut self, offset: usize, data: &[u8]) {
        if offset < 6 {
            let end = (offset + data.len()).min(6);
            for (i, &byte) in data.iter().enumerate().take(end - offset) {
                self.config.mac[offset + i] = byte;
            }
        }
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        let queue = &mut queues[0];
        self.tick_rx_queue(queue)
    }
}
