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
        0x02
    }

    fn features(&self) -> u32 {
        0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        if queue_sel != 0 {
            return false;
        }

        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut did_work = false;
        let mut count = 0;

        while let Some(head) = queue.pop_avail(guest_memory) {
            count += 1;
            let header = queue.get_descriptor(guest_memory, head);

            if header.flags & 1 == 0 || header.len != 16 {
                queue.push_used(guest_memory, head, 0);
                continue;
            }

            let request = BlkRequest::new(header.addr, guest_memory);
            let data_section = queue.get_descriptor(guest_memory, header.next);

            let flags_ok = if request.rqst_type {
                // write request: data is device-readable
                data_section.flags & 2 == 0
            } else {
                // read request: data is device-writable
                data_section.flags & 2 != 0
            };

            if !flags_ok {
                eprint!(
                    "blk: SKIP data flags={:#06x} len={} \r\n",
                    data_section.flags, data_section.len
                );
                queue.push_used(guest_memory, head, 0);
                continue;
            }

            let status_byte = queue.get_descriptor(guest_memory, data_section.next);

            if status_byte.flags & 2 == 0 {
                eprint!("blk: SKIP status flags={:#06x}\r\n", status_byte.flags);
                queue.push_used(guest_memory, head, 0);
                continue;
            }

            match request.rqst_type {
                false => {
                    if self
                        .blk_file
                        .seek(SeekFrom::Start(request.sector * SECTOR_SIZE))
                        .is_err()
                    {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }
                    let mut buf = vec![0u8; data_section.len as usize];

                    match self.blk_file.read_exact(&mut buf) {
                        Ok(_) => {
                            guest_memory.write_guest_memory(data_section.addr, &buf);
                            guest_memory.write_u8(status_byte.addr, 0x00);
                        }
                        Err(_) => {
                            guest_memory.write_u8(status_byte.addr, 0x01);
                        }
                    }
                }
                true => {
                    let mut buf = vec![0u8; data_section.len as usize];
                    guest_memory.read_guest_memory(data_section.addr, &mut buf);
                    if self
                        .blk_file
                        .seek(SeekFrom::Start(request.sector * 512))
                        .is_err()
                    {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }

                    match self.blk_file.write_all(&buf) {
                        Ok(_) => {
                            guest_memory.write_u8(status_byte.addr, 0x00);
                        }
                        Err(e) => {
                            eprint!("blk: WRITE sector={} err={}\n", request.sector, e);
                            guest_memory.write_u8(status_byte.addr, 0x01);
                        }
                    }
                }
            }

            queue.push_used(guest_memory, head, data_section.len);

            did_work = true;
        }

        if count > 10 {
            eprint!("blk: batch {} requests\r\n", count);
        }

        did_work
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, _queues: &mut [VirtioQueue]) -> bool {
        false
    }
}
