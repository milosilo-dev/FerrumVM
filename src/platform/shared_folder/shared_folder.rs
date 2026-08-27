use std::{
    collections::HashMap,
    fs::File,
    os::unix::fs::{DirEntryExt, MetadataExt},
    path::PathBuf,
};

use crate::platform::shared_folder::fuse::{
    header::FuseInHeader,
    init::{
        CONGESTION_THRESHOLD, FUSE_INIT_OUT_SIZE_PRE_MAX_PAGES, FUSE_INIT_OUT_SIZE_V0, FuseInitIn,
        FuseInitOut, MAX_BACKGROUND, MAX_READAHEAD, MAX_WRITE, TIME_GRAN_NS, truncate_init_out,
    },
    opcode::{
        FUSE_BATCH_FORGET, FUSE_CREATE, FUSE_DESTROY, FUSE_FLUSH, FUSE_FSYNC, FUSE_GETATTR,
        FUSE_INIT, FUSE_INTERRUPT, FUSE_LOOKUP, FUSE_MKDIR, FUSE_MKNOD, FUSE_OPEN, FUSE_OPENDIR,
        FUSE_READ, FUSE_READDIR, FUSE_READLINK, FUSE_RELEASE, FUSE_RELEASEDIR, FUSE_RENAME,
        FUSE_RMDIR, FUSE_ROOT_ID, FUSE_SETATTR, FUSE_STATFS, FUSE_SYMLINK, FUSE_UNLINK, FUSE_WRITE,
    },
};

pub struct SharedFolder {
    // FUSE "nodeid" -> host path. Root is always FUSE_ROOT_ID.
    inodes: HashMap<u64, PathBuf>,
    next_inode: u64,
    // FUSE file handle -> open host File, for READ/WRITE/RELEASE.
    open_files: HashMap<u64, File>,
    next_fh: u64,
    proto_minor: Option<u32>,
}

const FUSE_KERNEL_VERSION: u32 = 7;
const FUSE_KERNEL_MINOR_VERSION: u32 = 41;

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

    fn init(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    fn destroy(&mut self, _header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn make_node(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let mode = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);

        let name_start = 16; // after FuseMknodIn (mode + rdev + umask + padding)
        let name_end = body[name_start..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(body.len() - name_start)
            + name_start;
        let name = std::str::from_utf8(&body[name_start..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        if mode & libc::S_IFMT as u32 == libc::S_IFREG as u32 {
            std::fs::File::create(&child_path).map_err(|_| libc::EIO)?;
        } else {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::File::options()
                .create_new(true)
                .mode(mode & 0o7777)
                .open(&child_path)
                .map_err(|_| libc::EIO)?;
        }

        let nodeid = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(nodeid, child_path.clone());
        let meta = std::fs::metadata(&child_path).map_err(|_| libc::EIO)?;

        Ok(build_entry_out(nodeid, &meta))
    }

    fn make_directory(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let mode = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);

        let name_start = 8; // after FuseMkdirIn (mode + umask)
        let name_end = body[name_start..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(body.len() - name_start)
            + name_start;
        let name = std::str::from_utf8(&body[name_start..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        std::fs::create_dir(&child_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::AlreadyExists => libc::EEXIST,
            _ => libc::EIO,
        })?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(
                &child_path,
                std::fs::Permissions::from_mode(mode & 0o7777),
            );
        }

        let nodeid = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(nodeid, child_path);
        let meta = std::fs::metadata(self.inodes.get(&nodeid).unwrap()).map_err(|_| libc::EIO)?;

        Ok(build_entry_out(nodeid, &meta))
    }

    fn make_symlink(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let name_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
        let link_end = body[name_end + 1..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(body.len() - name_end - 1)
            + name_end
            + 1;

        let name = std::str::from_utf8(&body[..name_end]).map_err(|_| libc::EIO)?;
        let link_target =
            std::str::from_utf8(&body[name_end + 1..link_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        std::os::unix::fs::symlink(link_target, &child_path).map_err(|_| libc::EIO)?;

        let nodeid = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(nodeid, child_path);
        let meta =
            std::fs::symlink_metadata(self.inodes.get(&nodeid).unwrap()).map_err(|_| libc::EIO)?;

        Ok(build_entry_out(nodeid, &meta))
    }

    fn create(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let flags = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);

        let name_start = 16; // after FuseCreateIn (flags + mode + umask + padding)
        let name_end = body[name_start..]
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(body.len() - name_start)
            + name_start;
        let name = std::str::from_utf8(&body[name_start..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(flags & libc::O_TRUNC as u32 != 0)
            .open(&child_path)
            .map_err(|_| libc::EIO)?;

        let nodeid = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(nodeid, child_path);
        let meta = std::fs::metadata(self.inodes.get(&nodeid).unwrap()).map_err(|_| libc::EIO)?;

        let fh = self.next_fh;
        self.next_fh += 1;
        self.open_files.insert(fh, file);

        let mut entry_out = build_entry_out(nodeid, &meta);
        entry_out.extend(fh.to_le_bytes());
        entry_out.extend(0u32.to_le_bytes()); // open_flags
        entry_out.extend(0u32.to_le_bytes()); // padding
        Ok(entry_out)
    }

    fn open(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let flags = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);

        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;

        let access = flags & 0x3;
        let file = std::fs::OpenOptions::new()
            .read(access == libc::O_RDONLY as u32 || access == libc::O_RDWR as u32)
            .write(access == libc::O_WRONLY as u32 || access == libc::O_RDWR as u32)
            .append(flags & libc::O_APPEND as u32 != 0)
            .truncate(flags & libc::O_TRUNC as u32 != 0)
            .create(flags & libc::O_CREAT as u32 != 0)
            .open(path)
            .map_err(|e| match e.kind() {
                std::io::ErrorKind::NotFound => libc::ENOENT,
                std::io::ErrorKind::PermissionDenied => libc::EACCES,
                std::io::ErrorKind::AlreadyExists => libc::EEXIST,
                _ => libc::EIO,
            })?;

        let fh = self.next_fh;
        self.next_fh += 1;
        self.open_files.insert(fh, file);

        let mut out = Vec::new();
        out.extend(fh.to_le_bytes());
        out.extend(0u32.to_le_bytes()); // open_flags
        out.extend(0u32.to_le_bytes()); // padding
        Ok(out)
    }

    fn release(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        self.open_files.remove(&fh);
        Ok(vec![])
    }

    fn open_dir(&mut self, header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;
        let meta = std::fs::metadata(path).map_err(|_| libc::EIO)?;
        if !meta.is_dir() {
            return Err(libc::ENOTDIR);
        }

        let fh = self.next_fh;
        self.next_fh += 1;

        let mut out = Vec::new();
        out.extend(fh.to_le_bytes());
        out.extend(0u32.to_le_bytes()); // open_flags
        out.extend(0u32.to_le_bytes()); // padding
        Ok(out)
    }

    fn release_dir(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn set_attr(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let valid = u32::from_le_bytes(body[0..4].try_into().map_err(|_| libc::EIO)?);
        // body layout: valid(4) + padding(4) + fh(8) + size(8) + lock_owner(8) + atime(8) + mtime(8)
        //            + atimensec(4) + mtimensec(4) + ctime(4) + ctimensec(4) + mode(4)

        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;

        const FATTR_MODE: u32 = 1 << 0;
        const FATTR_SIZE: u32 = 1 << 3;

        if valid & FATTR_SIZE != 0 {
            let size = u64::from_le_bytes(body[16..24].try_into().map_err(|_| libc::EIO)?);
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(path)
                .map_err(|_| libc::EIO)?;
            file.set_len(size).map_err(|_| libc::EIO)?;
        }

        if valid & FATTR_MODE != 0 {
            let mode = u32::from_le_bytes(body[68..72].try_into().map_err(|_| libc::EIO)?);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let _ =
                    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode & 0o7777));
            }
        }

        let meta = std::fs::metadata(path).map_err(|_| libc::EIO)?;
        Ok(build_attr_out(&meta))
    }

    fn flush(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    fn fsync(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        if let Some(file) = self.open_files.get(&fh) {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::fsync(file.as_raw_fd());
            }
        }
        Ok(vec![])
    }

    fn stat_fs(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        let mut out = Vec::new();
        out.extend(0u64.to_le_bytes()); // blocks
        out.extend(0u64.to_le_bytes()); // bfree
        out.extend(0u64.to_le_bytes()); // bavail
        out.extend(0u64.to_le_bytes()); // files
        out.extend(0u64.to_le_bytes()); // ffree
        out.extend(0u32.to_le_bytes()); // bsize (filesystem block size)
        out.extend(0u32.to_le_bytes()); // namelen
        out.extend(0u32.to_le_bytes()); // frsize
        out.extend(0u32.to_le_bytes()); // padding
        out.extend(0u32.to_le_bytes()); // spare[0]
        out.extend(0u32.to_le_bytes()); // spare[1]
        out.extend(0u32.to_le_bytes()); // spare[2]
        out.extend(0u32.to_le_bytes()); // spare[3]
        out.extend(0u32.to_le_bytes()); // spare[4]
        out.extend(0u32.to_le_bytes()); // spare[5]
        Ok(out)
    }

    fn unlink(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let name_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
        let name = std::str::from_utf8(&body[..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        std::fs::remove_file(&child_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => libc::ENOENT,
            std::io::ErrorKind::PermissionDenied => libc::EACCES,
            std::io::ErrorKind::DirectoryNotEmpty => libc::ENOTEMPTY,
            _ => libc::EIO,
        })?;

        Ok(vec![])
    }

    fn rmdir(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let name_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
        let name = std::str::from_utf8(&body[..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        std::fs::remove_dir(&child_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => libc::ENOENT,
            std::io::ErrorKind::PermissionDenied => libc::EACCES,
            std::io::ErrorKind::DirectoryNotEmpty => libc::ENOTEMPTY,
            _ => libc::EIO,
        })?;

        Ok(vec![])
    }

    fn rename(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        // fuse_rename_in: newdir(8) + name + '\0' + newname + '\0'
        let newdir = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        let name_bytes = &body[8..];
        let name_end = name_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(name_bytes.len());
        let name = std::str::from_utf8(&name_bytes[..name_end]).map_err(|_| libc::EIO)?;
        let newname_bytes = &name_bytes[name_end + 1..];
        let newname_end = newname_bytes
            .iter()
            .position(|&b| b == 0)
            .unwrap_or(newname_bytes.len());
        let newname = std::str::from_utf8(&newname_bytes[..newname_end]).map_err(|_| libc::EIO)?;

        let old_parent = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let new_parent = self.inodes.get(&newdir).ok_or(libc::ENOENT)?;

        let old_path = old_parent.join(name);
        let new_path = new_parent.join(newname);

        std::fs::rename(&old_path, &new_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => libc::ENOENT,
            std::io::ErrorKind::PermissionDenied => libc::EACCES,
            _ => libc::EIO,
        })?;

        Ok(vec![])
    }

    fn readlink(&mut self, header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        let path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let target = std::fs::read_link(path).map_err(|_| libc::EIO)?;
        let target_bytes = target.to_string_lossy().as_bytes().to_vec();
        let mut out = target_bytes;
        out.push(0); // null terminator
        Ok(out)
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

    fn lookup(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let name_end = body.iter().position(|&b| b == 0).unwrap_or(body.len());
        let name = std::str::from_utf8(&body[..name_end]).map_err(|_| libc::EIO)?;

        let parent_path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let child_path = parent_path.join(name);

        let meta = std::fs::metadata(&child_path).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => libc::ENOENT,
            std::io::ErrorKind::PermissionDenied => libc::EACCES,
            _ => libc::EIO,
        })?;

        let nodeid = self.next_inode;
        self.next_inode += 1;
        self.inodes.insert(nodeid, child_path);

        Ok(build_entry_out(nodeid, &meta))
    }

    fn get_attr(&mut self, header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;
        let meta = std::fs::metadata(path).map_err(|_| libc::EIO)?;

        Ok(build_attr_out(&meta))
    }

    fn read(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        let offset = u64::from_le_bytes(body[8..16].try_into().map_err(|_| libc::EIO)?);
        let size = u32::from_le_bytes(body[16..20].try_into().map_err(|_| libc::EIO)?);

        use std::io::{Read, Seek, SeekFrom};
        let file = self.open_files.get_mut(&fh).ok_or(libc::EBADF)?;
        file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;

        let mut buf = vec![0u8; size as usize];
        let n = file.read(&mut buf).map_err(|_| libc::EIO)?;
        buf.truncate(n);
        Ok(buf)
    }

    fn write(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        let offset = u64::from_le_bytes(body[8..16].try_into().map_err(|_| libc::EIO)?);
        let size = u32::from_le_bytes(body[16..20].try_into().map_err(|_| libc::EIO)?);
        let data = &body[24..]; // 4 bytes writing_mode padding after size

        use std::io::{Seek, SeekFrom, Write};
        let file = self.open_files.get_mut(&fh).ok_or(libc::EBADF)?;
        file.seek(SeekFrom::Start(offset)).map_err(|_| libc::EIO)?;

        let write_len = (size as usize).min(data.len());
        file.write_all(&data[..write_len]).map_err(|_| libc::EIO)?;

        let mut out = Vec::new();
        out.extend((write_len as u32).to_le_bytes());
        out.extend(0u32.to_le_bytes()); // padding
        Ok(out)
    }

    fn read_dir(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let offset = u64::from_le_bytes(body[8..16].try_into().map_err(|_| libc::EIO)?);
        let max_size = u32::from_le_bytes(body[16..20].try_into().map_err(|_| libc::EIO)?);

        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;
        let entries: Vec<_> = std::fs::read_dir(path)
            .map_err(|_| libc::EIO)?
            .filter_map(|e| e.ok())
            .collect();

        let mut out = Vec::new();
        let mut pos: u64 = 1; // FUSE direntry offsets start at 1

        for entry in &entries {
            if pos <= offset {
                pos += 1;
                continue;
            }

            let name = entry.file_name();
            let name_bytes = name.to_string_lossy().as_bytes().to_vec();
            let name_len = name_bytes.len() as u32;

            let meta = entry.metadata().map_err(|_| libc::EIO)?;
            let file_type = if meta.is_dir() {
                4u32 // DT_DIR
            } else if meta.is_file() {
                8u32 // DT_REG
            } else if meta.is_symlink() {
                10u32 // DT_LNK
            } else {
                0u32 // DT_UNKNOWN
            };

            // Each direntry: ino(8) + off(8) + namelen(4) + type(4) + name + padding
            let reclen = 24 + name_len;
            let padded_reclen = (reclen + 7) & !7;

            if out.len() + padded_reclen as usize > max_size as usize {
                break;
            }

            let ino = entry.ino();

            out.extend(ino.to_le_bytes());
            out.extend(pos.to_le_bytes());
            out.extend(name_len.to_le_bytes());
            out.extend(file_type.to_le_bytes());
            out.extend(&name_bytes);
            out.resize(out.len() + (padded_reclen - reclen) as usize, 0);

            pos += 1;
        }

        Ok(out)
    }
}

fn fuse_attr_bytes(ino: u64, meta: &std::fs::Metadata) -> Vec<u8> {
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

fn build_entry_out(nodeid: u64, meta: &std::fs::Metadata) -> Vec<u8> {
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

fn build_attr_out(meta: &std::fs::Metadata) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend(1u64.to_le_bytes()); // attr_valid (seconds)
    out.extend(0u32.to_le_bytes()); // attr_valid_nsec
    out.extend(0u32.to_le_bytes()); // dummy
    out.extend(fuse_attr_bytes(0, meta));
    out
}
