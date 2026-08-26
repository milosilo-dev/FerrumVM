use crate::platform::shared_folder::fuse::util::read_le_u32;
use std::mem::size_of;

#[repr(C)]
pub struct FuseInitIn {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub flags2: u32,
}

impl FuseInitIn {
    pub fn new(bytes: Vec<u8>) -> (Self, Vec<u8>) {
        let input = &mut bytes.as_slice();

        (
            Self {
                major: read_le_u32(input),
                minor: read_le_u32(input),
                max_readahead: read_le_u32(input),
                flags: read_le_u32(input),
                flags2: read_le_u32(input),
            },
            {
                for _ in 0..11 {
                    read_le_u32(input);
                }

                input.to_vec()
            },
        )
    }
}

#[repr(C)]
#[derive(Default)]
pub struct FuseInitOut {
    pub major: u32,
    pub minor: u32,
    pub max_readahead: u32,
    pub flags: u32,
    pub flags2: u32,
    pub max_background: u16,
    pub congestion_threshold: u16,
    pub max_write: u32,
    pub time_gran: u32,
    pub max_pages: u16,
    pub map_alignment: u16,
}

impl FuseInitOut {
    pub fn new(
        major: u32,
        minor: u32,
        max_readahead: u32,
        flags: u32,
        flags2: u32,
        max_background: u16,
        congestion_threshold: u16,
        max_write: u32,
        time_gran: u32,
        max_pages: u16,
        map_alignment: u16,
    ) -> Self {
        Self {
            major,
            minor,
            max_readahead,
            flags,
            flags2,
            max_background,
            congestion_threshold,
            max_write,
            time_gran,
            max_pages,
            map_alignment,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = self.major.to_le_bytes().to_vec();

        buf.extend(self.minor.to_le_bytes());
        buf.extend(self.max_readahead.to_le_bytes());
        buf.extend(self.flags.to_le_bytes());
        buf.extend(self.flags2.to_le_bytes());
        buf.extend(self.max_background.to_le_bytes());
        buf.extend(self.congestion_threshold.to_le_bytes());
        buf.extend(self.max_write.to_le_bytes());
        buf.extend(self.time_gran.to_le_bytes());
        buf.extend(self.max_pages.to_le_bytes());
        buf.extend(self.map_alignment.to_le_bytes());

        buf.extend([0u8; size_of::<u32>() * 8]);

        buf
    }

    pub const fn length() -> usize {
        size_of::<u32>() * 5
            + size_of::<u16>() * 2
            + size_of::<u32>() * 2
            + size_of::<u16>() * 2
            + size_of::<u32>() * 8
    }
}

pub fn truncate_init_out(out: &FuseInitOut, len: usize) -> Vec<u8> {
    let mut bytes = out.to_bytes();
    bytes.truncate(len);
    bytes
}

pub const FUSE_COMPAT_INIT_OUT_SIZE: usize = size_of::<u32>() * 2;

pub const FUSE_COMPAT_22_INIT_OUT_SIZE: usize =
    size_of::<u32>() * 2 + size_of::<u32>() + size_of::<u32>();

pub const FUSE_INIT_OUT_SIZE_PRE_MAX_PAGES: usize =
    FuseInitOut::length() - size_of::<u16>() * 2 - size_of::<u32>() * 8;

pub const FUSE_INIT_OUT_SIZE_V0: usize = FUSE_COMPAT_INIT_OUT_SIZE;

pub const MAX_READAHEAD: u32 = 128 * 1024;
pub const MAX_BACKGROUND: u16 = 16;
pub const CONGESTION_THRESHOLD: u16 = 12;
pub const MAX_WRITE: u32 = 1024 * 1024;
pub const TIME_GRAN_NS: u32 = 1;
