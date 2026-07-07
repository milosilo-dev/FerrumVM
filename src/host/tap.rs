use libc::{c_short, ifreq};
use std::io::Error;
use std::mem;

const TUNSETIFF: libc::c_ulong = 0x400454ca;
const IFF_TAP: c_short = 0x0002;
const IFF_NO_PI: c_short = 0x1000;

pub fn create_tap(name: &str) -> std::fs::File {
    use std::ffi::CString;
    use std::os::unix::io::FromRawFd;

    let fd = unsafe { libc::open(b"/dev/net/tun\0".as_ptr() as *const i8, libc::O_RDWR) };

    if fd < 0 {
        let err = Error::last_os_error();
        panic!("failed to open /dev/net/tun: {err} (errno={})", err.raw_os_error().unwrap_or(0));
    }

    let mut req: ifreq = unsafe { mem::zeroed() };

    let cname = CString::new(name).unwrap();
    unsafe {
        std::ptr::copy_nonoverlapping(cname.as_ptr(), req.ifr_name.as_mut_ptr(), name.len());
    }

    let flags: c_short = (IFF_TAP | IFF_NO_PI) as c_short;
    unsafe {
        std::ptr::write(
            &mut req.ifr_ifru.ifru_flags as *mut _ as *mut c_short,
            flags,
        );
    }

    unsafe {
        let flags_ptr = &mut req.ifr_ifru as *mut _ as *mut c_short;
        *flags_ptr = (IFF_TAP | IFF_NO_PI) as c_short;
    }

    let res = unsafe { libc::ioctl(fd, TUNSETIFF, &req) };

    if res < 0 {
        let err = Error::last_os_error();
        panic!("TUNSETIFF failed: {err} (errno={})", err.raw_os_error().unwrap_or(0));
    }

    unsafe { std::fs::File::from_raw_fd(fd) }
}
