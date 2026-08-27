use crate::platform::shared_folder::{
    fuse::header::FuseInHeader,
    util::{build_attr_out, build_entry_out},
};

use super::SharedFolder;

impl SharedFolder {
    pub(crate) fn lookup(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn get_attr(&mut self, header: &FuseInHeader) -> Result<Vec<u8>, i32> {
        let path = self.inodes.get(&header.nodeid).ok_or(libc::EIO)?;
        let meta = std::fs::metadata(path).map_err(|_| libc::EIO)?;

        Ok(build_attr_out(&meta))
    }

    pub(crate) fn make_node(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn make_directory(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn make_symlink(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn set_attr(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn stat_fs(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn unlink(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn rmdir(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn rename(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn readlink(&mut self, header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        let path = self.inodes.get(&header.nodeid).ok_or(libc::ENOENT)?;
        let target = std::fs::read_link(path).map_err(|_| libc::EIO)?;
        let target_bytes = target.to_string_lossy().as_bytes().to_vec();
        let mut out = target_bytes;
        out.push(0); // null terminator
        Ok(out)
    }
}
