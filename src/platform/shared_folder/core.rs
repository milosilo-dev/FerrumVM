use std::{collections::HashMap, fs::File, path::PathBuf};

use crate::platform::shared_folder::fuse::{
    header::FuseInHeader,
    opcode::{
        FUSE_BATCH_FORGET, FUSE_CREATE, FUSE_DESTROY, FUSE_FLUSH, FUSE_FSYNC, FUSE_GETATTR,
        FUSE_INIT, FUSE_INTERRUPT, FUSE_LOOKUP, FUSE_MKDIR, FUSE_MKNOD, FUSE_OPEN, FUSE_OPENDIR,
        FUSE_READ, FUSE_READDIR, FUSE_READLINK, FUSE_RELEASE, FUSE_RELEASEDIR, FUSE_RENAME,
        FUSE_RMDIR, FUSE_ROOT_ID, FUSE_SETATTR, FUSE_STATFS, FUSE_SYMLINK, FUSE_UNLINK, FUSE_WRITE,
    },
};

pub struct SharedFolder {
    // FUSE "nodeid" -> host path. Root is always FUSE_ROOT_ID.
    pub(crate) inodes: HashMap<u64, PathBuf>,
    pub(crate) next_inode: u64,
    // FUSE file handle -> open host File, for READ/WRITE/RELEASE.
    pub(crate) open_files: HashMap<u64, File>,
    pub(crate) next_fh: u64,
    pub(crate) proto_minor: Option<u32>,
}

impl SharedFolder {
    pub fn new(root: PathBuf) -> Self {
        let mut inodes = HashMap::new();
        inodes.insert(FUSE_ROOT_ID, root.clone());
        Self {
            inodes,
            next_inode: FUSE_ROOT_ID + 1,
            open_files: HashMap::new(),
            next_fh: 1,
            proto_minor: None,
        }
    }

    pub fn dispatch(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        eprintln!(
            "FUSE: opcode={} nodeid={} unique={}",
            header.opcode, header.nodeid, header.unique
        );
        let result = match header.opcode {
            FUSE_INIT => self.init(header, body),
            FUSE_DESTROY => self.destroy(header),
            FUSE_LOOKUP => self.lookup(header, body),
            FUSE_GETATTR => self.get_attr(header),
            FUSE_READ => self.read(header, body),
            FUSE_MKNOD => self.make_node(header, body),
            FUSE_MKDIR => self.make_directory(header, body),
            FUSE_SYMLINK => self.make_symlink(header, body),
            FUSE_CREATE => self.create(header, body),
            FUSE_WRITE => self.write(header, body),
            FUSE_RELEASE => self.release(header, body),
            FUSE_OPENDIR => self.open_dir(header, body),
            FUSE_RELEASEDIR => self.release_dir(header, body),
            FUSE_READDIR => self.read_dir(header, body),
            FUSE_SETATTR => self.set_attr(header, body),
            FUSE_FLUSH => self.flush(header, body),
            FUSE_FSYNC => self.fsync(header, body),
            FUSE_OPEN => self.open(header, body),
            FUSE_STATFS => self.stat_fs(header, body),
            FUSE_UNLINK => self.unlink(header, body),
            FUSE_RMDIR => self.rmdir(header, body),
            FUSE_RENAME => self.rename(header, body),
            FUSE_READLINK => self.readlink(header, body),
            FUSE_INTERRUPT => self.interrupt(header, body),
            FUSE_BATCH_FORGET => self.batch_forget(header, body),
            _ => {
                eprintln!("FUSE: unsupported opcode {}", header.opcode);
                Err(libc::ENOSYS)
            }
        };
        if let Err(e) = &result {
            eprintln!("FUSE: opcode={} failed with errno={}", header.opcode, e);
        }
        result
    }

    pub fn forget(&mut self, header: &FuseInHeader) {
        self.inodes.remove(&header.nodeid);
    }

    fn interrupt(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn batch_forget(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        // fuse_batch_forget_in: count(4) + dummy(4)
        let count = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);
        // fuse_forget_one: nodeid(8) + nlookup(8)
        for i in 0..count as usize {
            let offset = 8 + i * 16;
            if offset + 8 > body.len() {
                break;
            }
            let nodeid =
                u64::from_le_bytes(body[offset..offset + 8].try_into().map_err(|_| libc::EIO)?);
            self.inodes.remove(&nodeid);
        }
        Ok(vec![])
    }
}
