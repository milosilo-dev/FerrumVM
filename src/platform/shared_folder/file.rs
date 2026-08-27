use crate::platform::shared_folder::{
    fuse::header::FuseInHeader,
    util::build_entry_out,
};

use super::SharedFolder;

impl SharedFolder {
    pub(crate) fn create(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn open(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn release(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        self.open_files.remove(&fh);
        Ok(vec![])
    }

    pub(crate) fn read(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn write(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn flush(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    pub(crate) fn fsync(&mut self, _header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
        let fh = u64::from_le_bytes(body[0..8].try_into().map_err(|_| libc::EIO)?);
        if let Some(file) = self.open_files.get(&fh) {
            use std::os::fd::AsRawFd;
            unsafe {
                libc::fsync(file.as_raw_fd());
            }
        }
        Ok(vec![])
    }
}
