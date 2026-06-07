use std::{
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom, Write},
    path::Path,
};

use crate::devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue};

const SECTOR_SIZE: u64 = 512;

pub struct BlkRequest {
    pub rqst_type: bool,
    pub sector: u64,
}

impl BlkRequest {
    pub fn new(base_ptr: u64, guest_memory: &VirtioGuestMemoryHandle) -> Self {
        let rqst_type = guest_memory.read_u32(base_ptr) != 0;
        let sector = guest_memory.read_u64(base_ptr + 8);
        Self { rqst_type, sector }
    }
}

struct VirtioBlkConfig {
    capacity: u64,
    size_max: u32,
    seg_max: u32,
    blk_size: u32,
}

impl VirtioBlkConfig {
    pub fn new(capacity: u64, size_max: u32, seg_max: u32, blk_size: u32) -> Self {
        Self {
            capacity: capacity / blk_size as u64,
            size_max,
            seg_max,
            blk_size,
        }
    }

    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = self.capacity.to_le_bytes().to_vec();
        buf.extend(self.size_max.to_le_bytes());
        buf.extend(self.seg_max.to_le_bytes());
        buf.extend(self.blk_size.to_le_bytes());

        buf.resize(length, 0);

        buf
    }
}

pub struct BlkVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    blk_file: File,
    config: VirtioBlkConfig,
}

impl BlkVirtio {
    pub fn new(blk_file: &str) -> Self {
        let path = Path::new(blk_file);
        if !path.exists() {
            File::create(path).unwrap();
        }

        let blk_file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("failed to open disk image");
        let file_size = blk_file.metadata().unwrap().len();

        Self {
            guest_memory: None,
            blk_file,
            config: VirtioBlkConfig::new(file_size, 1024 * 1024, 128, 512),
        }
    }
}

impl VirtioDevice for BlkVirtio {
    fn virtio_type(&self) -> u32 {
        0x04
    }

    fn features(&self) -> u32 {
        0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue: &mut VirtioQueue) -> bool {
        if self.guest_memory.is_none() {
            return false;
        }

        let mut guest_memory = self.guest_memory.as_mut().unwrap();
        let mut did_work = false;

        while let Some(head) = queue.pop_avail(&guest_memory) {
            let header = queue.get_descriptor(&guest_memory, head);

            if header.flags & 1 == 0 {
                // Next
                eprintln!(
                    "virtio-blk: head={}, addr=0x{:x}, len={}, flags=0x{:x}, next={}",
                    head, header.addr, header.len, header.flags, header.next
                );
                eprintln!(
                    "virtio-blk: desc_addr=0x{:x}, avail_addr=0x{:x}, used_addr=0x{:x}",
                    queue.desc_addr, queue.avail_addr, queue.used_addr
                );
                eprintln!(
                    "virtio-blk: last_avail_idx={}, queue.size={}, queue.ready={}",
                    queue.last_avail_idx, queue.size, queue.ready
                );
                let avail_idx = guest_memory.read_u16(queue.avail_addr + 2);
                eprintln!("virtio-blk: avail_idx={}", avail_idx);
                for i in 0..16 {
                    let off = 4 + i * 2;
                    let v = guest_memory.read_u16(queue.avail_addr + off);
                    eprintln!("virtio-blk:   ring[{}] = {}", i, v);
                }
                // Dump descriptor table first 8 entries
                for i in 0..8 {
                    let d = queue.get_descriptor(&guest_memory, i);
                    eprintln!(
                        "virtio-blk:   desc[{}] addr=0x{:x} len={} flags={} next={}",
                        i, d.addr, d.len, d.flags, d.next
                    );
                }
                panic!("virtio-blk got inncorect header");
            }

            if header.len != 16 {
                panic!("virtio-blk got the wrong header length");
            }

            let request = BlkRequest::new(header.addr, &guest_memory);

            let data_section = queue.get_descriptor(&guest_memory, header.next);

            if data_section.flags & 1 == 0 || data_section.flags & 2 == 0 {
                // Next & Write
                panic!("virtio-blk got inncorect data buffer");
            }

            let status_byte = queue.get_descriptor(&guest_memory, data_section.next);

            if status_byte.flags & 2 == 0 {
                panic!("virtio-blk got inncorect status_byte");
            }

            match request.rqst_type {
                false => {
                    // Device read
                    self.blk_file
                        .seek(SeekFrom::Start(request.sector * SECTOR_SIZE))
                        .unwrap();
                    let mut buf = vec![0u8; data_section.len as usize];

                    match self.blk_file.read_exact(&mut buf) {
                        Ok(_) => {
                            guest_memory.write_guest_memory(data_section.addr, &buf);
                            guest_memory.write_u8(status_byte.addr, 0x00);
                        }
                        Err(err) => {
                            println!("{}", err);
                            guest_memory.write_u8(status_byte.addr, 0x01)
                        }
                    }
                }
                true => {
                    // Device write
                    let mut buf = vec![0u8; data_section.len as usize];
                    guest_memory.read_guest_memory(data_section.addr, &mut buf);
                    self.blk_file
                        .seek(SeekFrom::Start(request.sector * 512))
                        .unwrap();

                    match self.blk_file.write_all(&buf) {
                        Ok(_) => guest_memory.write_u8(status_byte.addr, 0x00),
                        Err(_) => guest_memory.write_u8(status_byte.addr, 0x01),
                    }
                }
            }

            queue.push_used(&mut guest_memory, head, data_section.len);

            did_work = true;
        }

        did_work
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }
}
