use std::{collections::HashMap, fs::File, path::PathBuf};

use crate::platform::shared_folder::fuse::{
    FUSE_CREATE, FUSE_DESTROY, FUSE_GETATTR, FUSE_INIT, FUSE_LOOKUP, FUSE_MKDIR, FUSE_MKNOD, FUSE_READ, FUSE_ROOT_ID, FUSE_SYMLINK, FuseInHeader, FuseInitIn,
};

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
            FUSE_INIT => self.init(header, body),
            FUSE_DESTROY => self.destroy(header),
            FUSE_LOOKUP => self.lookup(header, body),
            FUSE_GETATTR => self.get_attr(header),
            FUSE_READ => self.read(header, body),
            FUSE_MKNOD => self.make_node(header, body),
            FUSE_MKDIR => self.make_directory(header, body),
            FUSE_SYMLINK => self.make_symlink(header, body),
            FUSE_CREATE => self.create(header, body),
            _ => Err(libc::ENOSYS),
        }
    }

    pub fn forget(&mut self, header: &FuseInHeader) {
        self.inodes.remove(&header.nodeid);
    }

    fn init(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let (init_data, body) = FuseInitIn::new(body.to_vec());
        

        Ok(vec![])
    }

    fn destroy(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn make_node(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn make_directory(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn make_symlink(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn create(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn open(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn release(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn open_dir(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn release_dir(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn lookup(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn get_attr(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn set_attr(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn flush(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn fsync(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn read(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn write(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn read_dir(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }
}
