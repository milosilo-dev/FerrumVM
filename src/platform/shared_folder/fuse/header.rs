use crate::platform::shared_folder::fuse::util::{read_le_u32, read_le_u64};
use std::mem::size_of;

#[repr(C)]
pub struct FuseInHeader {
    pub len: u32,
    pub opcode: u32,
    pub unique: u64,
    pub nodeid: u64,
    pub uid: u32,
    pub gid: u32,
    pub pid: u32,
    pub padding: u32,
}

impl FuseInHeader {
    pub fn new(bytes: Vec<u8>) -> (Self, Vec<u8>) {
        let input = &mut bytes.as_slice();

        (
            Self {
                len: read_le_u32(input),
                opcode: read_le_u32(input),
                unique: read_le_u64(input),
                nodeid: read_le_u64(input),
                uid: read_le_u32(input),
                gid: read_le_u32(input),
                pid: read_le_u32(input),
                padding: read_le_u32(input),
            },
            input.to_vec(),
        )
    }
}

#[repr(C)]
pub struct FuseOutHeader {
    pub len: u32,
    pub error: i32,
    pub unique: u64,
}

impl FuseOutHeader {
    pub fn new(len: u32, error: i32, unique: u64) -> Self {
        Self { len, error, unique }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.len.to_le_bytes().to_vec();
        buf.extend(self.error.to_le_bytes());
        buf.extend(self.unique.to_le_bytes());
        buf
    }

    pub fn length() -> usize {
        size_of::<u32>() + size_of::<i32>() + size_of::<u64>()
    }
}
