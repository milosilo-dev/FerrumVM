use std::{collections::HashMap, fs::File, path::PathBuf};

use crate::platform::shared_folder::fuse::{FuseInHeader, FUSE_LOOKUP, FUSE_GETATTR, FUSE_READ, FUSE_ROOT_ID};

pub struct SharedFolder {
    root: PathBuf,
    // FUSE "nodeid" -> host path. Root is always FUSE_ROOT_ID.
    inodes: HashMap<u64, PathBuf>,
    next_inode: u64,
    // FUSE file handle -> open host File, for READ/WRITE/RELEASE.
    open_files: HashMap<u64, File>,
    next_fh: u64,
}

impl SharedFolder {
    pub fn new(root: PathBuf) -> Self {
        let mut inodes = HashMap::new();
        inodes.insert(FUSE_ROOT_ID, root.clone());
        Self {
            root,
            inodes,
            next_inode: FUSE_ROOT_ID + 1,
            open_files: HashMap::new(),
            next_fh: 1,
        }
    }

    pub fn dispatch(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        match header.opcode {
            FUSE_LOOKUP => self.lookup(header, body),
            FUSE_GETATTR => self.getattr(header),
            FUSE_READ => self.read(header, body),
            _ => Err(libc::ENOSYS),
        }
    }

    pub fn forget(&mut self, header: &FuseInHeader) {
        self.inodes.remove(&header.nodeid);
    }

    pub fn lookup(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    pub fn getattr(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }
    
    pub fn read(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }
}
