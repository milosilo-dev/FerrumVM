use std::os::unix::fs::MetadataExt;

pub(crate) fn fuse_attr_bytes(ino: u64, meta: &std::fs::Metadata) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(ino.to_le_bytes());
    out.extend(meta.len().to_le_bytes());
    out.extend(((meta.len() + 511) / 512).to_le_bytes()); // blocks
    out.extend((meta.atime() as u64).to_le_bytes());
    out.extend((meta.mtime() as u64).to_le_bytes());
    out.extend((meta.ctime() as u64).to_le_bytes());
    out.extend((meta.atime_nsec() as u32).to_le_bytes());
    out.extend((meta.mtime_nsec() as u32).to_le_bytes());
    out.extend((meta.ctime_nsec() as u32).to_le_bytes());
    out.extend((meta.mode() as u32).to_le_bytes());
    out.extend((meta.nlink() as u32).to_le_bytes());
    out.extend((meta.uid()).to_le_bytes());
    out.extend((meta.gid()).to_le_bytes());
    out.extend((meta.rdev() as u32).to_le_bytes());
    out.extend((512u32).to_le_bytes()); // blksize
    out.extend(0u32.to_le_bytes()); // padding
    out
}

pub(crate) fn build_entry_out(nodeid: u64, meta: &std::fs::Metadata) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(nodeid.to_le_bytes()); // nodeid
    out.extend(0u64.to_le_bytes()); // generation
    out.extend(1u64.to_le_bytes()); // entry_valid (seconds)
    out.extend(1u64.to_le_bytes()); // attr_valid (seconds)
    out.extend(0u32.to_le_bytes()); // entry_valid_nsec
    out.extend(0u32.to_le_bytes()); // attr_valid_nsec
    out.extend(fuse_attr_bytes(nodeid, meta));
    out
}

pub(crate) fn build_attr_out(meta: &std::fs::Metadata) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(1u64.to_le_bytes()); // attr_valid (seconds)
    out.extend(0u32.to_le_bytes()); // attr_valid_nsec
    out.extend(0u32.to_le_bytes()); // dummy
    out.extend(fuse_attr_bytes(0, meta));
    out
}
