use std::ffi::CString;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::os::unix::fs::OpenOptionsExt;

fn main() {
    let fd = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(libc::O_NONBLOCK)
        .open("/dev/net/tun")
        .expect("Failed to open /dev/net/tun");

    let mut ifr = [0u8; 64];
    let cname = CString::new("ferrum-tap0").unwrap();
    let name_bytes = cname.as_bytes_with_nul();
    ifr[..name_bytes.len()].copy_from_slice(name_bytes);
    let flags = (libc::IFF_TAP | libc::IFF_NO_PI) as u16;
    ifr[16..18].copy_from_slice(&flags.to_le_bytes());

    let ret = unsafe {
        libc::ioctl(
            fd.as_raw_fd(),
            libc::TUNSETIFF,
            &ifr as *const _ as *const libc::c_void,
        )
    };
    if ret < 0 {
        eprint!("TUNSETIFF failed: {}\n", std::io::Error::last_os_error());
    }
}
