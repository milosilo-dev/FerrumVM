use crate::{
    devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtqDesc},
    platform::shared_folder::{
        fuse::{
            header::{FuseInHeader, FuseOutHeader},
            opcode::FUSE_FORGET,
        },
        shared_folder::SharedFolder,
    },
};

pub struct FsVirtioConfig {
    tag: Vec<u8>,
    request_queues: u32,
}

impl FsVirtioConfig {
    pub fn new(tag: &str, request_queues: u32) -> Self {
        let mut tag = tag.as_bytes().to_vec();
        tag.resize(36, 0x0);
        Self {
            tag: tag,
            request_queues,
        }
    }

    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = self.tag.clone();
        buf.extend(self.request_queues.to_le_bytes());

        buf.resize(length, 0);

        buf
    }
}

pub struct FsVirtio {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: FsVirtioConfig,
    fuse_device: SharedFolder,
}

impl FsVirtio {
    pub fn new(name: &str, fuse_device: SharedFolder) -> Self {
        Self {
            guest_memory: None,
            config: FsVirtioConfig::new(name, 1),
            fuse_device,
        }
    }
}

impl VirtioDevice for FsVirtio {
    fn virtio_type(&self) -> u32 {
        0x1A
    }

    fn features(&self) -> u64 {
        0x0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(
        &mut self,
        queue_sel: usize,
        queue: &mut crate::devices::virtio::virtio::VirtioQueue,
    ) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut did_work: bool = false;
        match queue_sel {
            0 => {
                // Hiprio
                while let Some(head) = queue.pop_avail(guest_memory) {
                    let fuse_in_descriptor = queue.get_descriptor(guest_memory, head);
                    let mut fuse_in_bytes = vec![0u8; fuse_in_descriptor.len as usize];
                    guest_memory.read_guest_memory(fuse_in_descriptor.addr, &mut fuse_in_bytes);
                    let (fuse_in_header, _) = FuseInHeader::new(fuse_in_bytes);

                    match fuse_in_header.opcode {
                        FUSE_FORGET => {
                            self.fuse_device.forget(&fuse_in_header);
                        }
                        _ => {}
                    }

                    queue.push_used(guest_memory, head, 0);
                    did_work = true;
                }
            }
            1 => {
                // Request queue
                while let Some(head) = queue.pop_avail(guest_memory) {
                    let fuse_in_descriptor = queue.get_descriptor(guest_memory, head);
                    let mut fuse_in_bytes = vec![0u8; fuse_in_descriptor.len as usize];
                    guest_memory.read_guest_memory(fuse_in_descriptor.addr, &mut fuse_in_bytes);
                    let (fuse_in_header, fuse_data) = FuseInHeader::new(fuse_in_bytes);

                    let mut out_desc: VirtqDesc =
                        queue.get_descriptor(guest_memory, fuse_in_descriptor.next);
                    while out_desc.flags & 2 == 0 {
                        out_desc = queue.get_descriptor(guest_memory, out_desc.next);
                    }

                    let reply_bytes = match self.fuse_device.dispatch(&fuse_in_header, &fuse_data) {
                        Ok(payload) => {
                            let out_header = FuseOutHeader {
                                len: (FuseOutHeader::length() + payload.len()) as u32,
                                error: 0,
                                unique: fuse_in_header.unique,
                            };
                            let mut bytes = out_header.to_bytes();
                            bytes.extend(payload);
                            bytes
                        }
                        Err(errno) => {
                            let out_header = FuseOutHeader {
                                len: FuseOutHeader::length() as u32,
                                error: -errno, // FUSE wants negative errno
                                unique: fuse_in_header.unique,
                            };
                            out_header.to_bytes()
                        }
                    };

                    let write_len = reply_bytes.len().min(out_desc.len as usize);
                    guest_memory.write_guest_memory(out_desc.addr, &reply_bytes[..write_len]);

                    queue.push_used(guest_memory, head, 0);
                    did_work = true;
                }
            }
            _ => {}
        }

        did_work
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, _queues: &mut [crate::devices::virtio::virtio::VirtioQueue]) -> bool {
        false
    }
}
