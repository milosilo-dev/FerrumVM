use std::os::unix::fs::DirEntryExt;

use crate::platform::shared_folder::fuse::header::FuseInHeader;

use super::SharedFolder;

impl SharedFolder {
    pub(crate) fn open_dir(&mut self, header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
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

    pub(crate) fn release_dir(&mut self, _header: &FuseInHeader, _body: &[u8]) -> Result<Vec<u8>, i32> {
        Ok(vec![])
    }

    pub(crate) fn read_dir(&mut self, header: &FuseInHeader, body: &[u8]) -> Result<Vec<u8>, i32> {
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
