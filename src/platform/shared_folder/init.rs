use crate::platform::shared_folder::fuse::{
    header::FuseInHeader,
    init::{
        CONGESTION_THRESHOLD, FUSE_INIT_OUT_SIZE_PRE_MAX_PAGES, FUSE_INIT_OUT_SIZE_V0, FuseInitIn,
        FuseInitOut, MAX_BACKGROUND, MAX_READAHEAD, MAX_WRITE, TIME_GRAN_NS, truncate_init_out,
    },
};

use super::SharedFolder;

const FUSE_KERNEL_VERSION: u32 = 7;
const FUSE_KERNEL_MINOR_VERSION: u32 = 41;

impl SharedFolder {
    pub(crate) fn init(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        eprintln!(
            "FUSE_INIT: body len={}, first 32 bytes: {:02x?}",
            body.len(),
            &body[..body.len().min(32)]
        );
        let (req, _rest) = FuseInitIn::new(body.to_vec());
        eprintln!(
            "FUSE_INIT: major={} minor={} max_readahead={} flags=0x{:x}",
            req.major, req.minor, req.max_readahead, req.flags
        );

        if req.major != FUSE_KERNEL_VERSION {
            if req.major > FUSE_KERNEL_VERSION {
                let out = FuseInitOut {
                    major: FUSE_KERNEL_VERSION,
                    minor: FUSE_KERNEL_MINOR_VERSION,
                    ..Default::default()
                };
                return Ok(truncate_init_out(&out, FUSE_INIT_OUT_SIZE_V0));
            } else {
                return Err(libc::EPROTO);
            }
        }

        let minor = req.minor.min(FUSE_KERNEL_MINOR_VERSION);
        self.proto_minor = Some(minor);

        let out_len = if minor < 5 {
            FUSE_INIT_OUT_SIZE_V0
        } else if minor < 23 {
            FUSE_INIT_OUT_SIZE_PRE_MAX_PAGES
        } else {
            std::mem::size_of::<FuseInitOut>()
        };

        let flags_out = self.negotiate_flags(req.flags);

        let out = FuseInitOut {
            major: FUSE_KERNEL_VERSION,
            minor,
            max_readahead: req.max_readahead.min(MAX_READAHEAD),
            flags: flags_out,
            max_background: MAX_BACKGROUND,
            congestion_threshold: CONGESTION_THRESHOLD,
            max_write: MAX_WRITE,
            time_gran: TIME_GRAN_NS,
            ..Default::default()
        };

        Ok(truncate_init_out(&out, out_len))
    }

    fn negotiate_flags(&self, guest_flags: u32) -> u32 {
        const SUPPORTED: u32 = 0; // start at 0, add bits as opcodes are implemented
        SUPPORTED & guest_flags
    }

    pub(crate) fn destroy(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }
}
