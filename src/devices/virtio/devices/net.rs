use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    sync::{Arc, Mutex},
};

use crate::{
    devices::virtio::virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue},
    memory_region::MemoryRegion,
};

// ---------------------------------------------------------------------------
// Vhost-net ioctl numbers (x86_64 Linux encoding)
// ---------------------------------------------------------------------------
const VHOST_SET_OWNER: libc::c_ulong = 0x0000_AF01;
const VHOST_SET_VRING_NUM: libc::c_ulong = 0x4008_AF10;
const VHOST_SET_VRING_ADDR: libc::c_ulong = 0x4028_AF11;
const VHOST_SET_VRING_BASE: libc::c_ulong = 0x4008_AF12;
const VHOST_SET_VRING_KICK: libc::c_ulong = 0x4008_AF20;
const VHOST_SET_VRING_CALL: libc::c_ulong = 0x4008_AF21;
const VHOST_NET_SET_BACKEND: libc::c_ulong = 0x4008_AF30;
const VHOST_SET_MEM_TABLE: libc::c_ulong = 0x4008_AF03;

// ---------------------------------------------------------------------------
// Virtio-net constants
// ---------------------------------------------------------------------------
const MAC_ADDRESS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
const VIRTIO_NET_F_MAC: u32 = 1 << 5;
const VIRTIO_NET_F_STATUS: u32 = 1 << 16;
const VIRTIO_NET_S_LINK_UP: u16 = 1;

// ---------------------------------------------------------------------------
// Vhost ABI structures (must match kernel structs)
// ---------------------------------------------------------------------------
#[repr(C)]
struct vhost_vring_state {
    index: u32,
    num: u32,
}

#[repr(C)]
struct vhost_vring_file {
    index: u32,
    fd: i32,
}

#[repr(C)]
struct vhost_vring_addr {
    index: u32,
    flags: u32,
    desc_user_addr: u64,
    used_user_addr: u64,
    avail_user_addr: u64,
    log_guest_addr: u64,
}

// ---------------------------------------------------------------------------
// Config space
// ---------------------------------------------------------------------------
pub struct VirtioNetConfig {
    mac: [u8; 6],
    status: u16,
    max_virtqueue_pairs: u16,
    mtu: u16,
}

impl VirtioNetConfig {
    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = Vec::with_capacity(length);
        buf.extend_from_slice(&self.mac);
        buf.extend_from_slice(&self.status.to_le_bytes());
        buf.extend_from_slice(&self.max_virtqueue_pairs.to_le_bytes());
        buf.extend_from_slice(&self.mtu.to_le_bytes());
        buf.resize(length, 0);
        buf
    }
}

// ---------------------------------------------------------------------------
// Vhost-net device
// ---------------------------------------------------------------------------
pub struct NetVirtio {
    vhost: File,
    tap: File,
    config: VirtioNetConfig,
    guest_memory: Option<VirtioGuestMemoryHandle>,
    mem_regions: Option<Arc<Mutex<Vec<MemoryRegion>>>>,
    kick_evt: [Option<File>; 2],
    call_evt: [Option<File>; 2],
    configured: bool,
    last_used_idx: [u16; 2],
}

impl NetVirtio {
    pub fn new(tap_name: &str) -> Self {
        let tap = Self::open_tap(tap_name);
        let vhost = Self::open_vhost();

        // Take ownership immediately.
        let ret = unsafe { libc::ioctl(vhost.as_raw_fd(), VHOST_SET_OWNER) };
        assert!(ret >= 0, "VHOST_SET_OWNER failed");

        Self {
            vhost,
            tap,
            config: VirtioNetConfig {
                mac: MAC_ADDRESS,
                status: VIRTIO_NET_S_LINK_UP,
                max_virtqueue_pairs: 1,
                mtu: 1500,
            },
            guest_memory: None,
            mem_regions: None,
            kick_evt: [None, None],
            call_evt: [None, None],
            configured: false,
            last_used_idx: [0, 0],
        }
    }

    // -- helpers -----------------------------------------------------------

    fn open_tap(name: &str) -> File {
        const TUNSETIFF: libc::c_ulong = 0x4004_54ca;
        const IFF_TAP: libc::c_short = 0x0002;
        const IFF_NO_PI: libc::c_short = 0x1000;
        const IFF_VNET_HDR: libc::c_short = 0x4000;

        let fd = unsafe {
            libc::open(
                b"/dev/net/tun\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR | libc::O_NONBLOCK,
            )
        };
        assert!(fd >= 0, "failed to open /dev/net/tun");

        let mut ifr = [0u8; 40];
        let name_bytes = name.as_bytes();
        let len = name_bytes.len().min(libc::IFNAMSIZ as usize - 1);
        ifr[..len].copy_from_slice(&name_bytes[..len]);
        let flags = IFF_TAP | IFF_NO_PI | IFF_VNET_HDR;
        ifr[16..18].copy_from_slice(&flags.to_le_bytes());

        let ret = unsafe {
            libc::ioctl(fd, TUNSETIFF, &ifr as *const _ as *const libc::c_void)
        };
        assert!(
            ret >= 0,
            "TUNSETIFF failed for '{}' (errno: {}, run scripts/enable_tap.sh first, \
             or use cargo-run.sh for cap_net_admin)",
            name,
            unsafe { *libc::__errno_location() },
        );

        unsafe { File::from_raw_fd(fd) }
    }

    fn open_vhost() -> File {
        let fd = unsafe {
            libc::open(
                b"/dev/vhost-net\0".as_ptr() as *const libc::c_char,
                libc::O_RDWR,
            )
        };
        assert!(
            fd >= 0,
            "failed to open /dev/vhost-net (errno: {})",
            unsafe { *libc::__errno_location() },
        );
        unsafe { File::from_raw_fd(fd) }
    }

    fn make_eventfd() -> File {
        let fd = unsafe { libc::eventfd(0, libc::EFD_NONBLOCK) };
        assert!(fd >= 0, "eventfd failed (errno: {})", unsafe {
            *libc::__errno_location()
        });
        unsafe { File::from_raw_fd(fd) }
    }

    /// Translate a guest-physical address to a host-virtual address using the
    /// memory regions provided by the VMM.
    fn gpa_to_hva(&self, gpa: u64) -> Option<u64> {
        let regions = self.mem_regions.as_ref()?;
        let borrow = regions.lock().ok()?;
        for region in borrow.iter() {
            let start = region.mem_offset;
            let end = region.mem_offset + region.mem_size as u64;
            if gpa >= start && gpa < end {
                return Some(region.ptr as u64 + (gpa - start));
            }
        }
        None
    }

    /// Write a 1 to the kick eventfd so the vhost worker thread wakes up and
    /// processes the given virtqueue.
    fn kick_vhost(_queue_idx: usize, evt: &Option<File>) {
        if let Some(f) = evt {
            let val = 1u64;
            unsafe {
                libc::write(f.as_raw_fd(), &val as *const _ as *const libc::c_void, 8);
            }
        }
    }

    // -- vhost setup -------------------------------------------------------

    /// Share the guest memory layout with the vhost kernel subsystem.
    fn set_mem_table(&self) {
        let Some(ref regions) = self.mem_regions else {
            return;
        };
        let borrow = regions.lock().unwrap();
        let nregions = borrow.len() as u32;

        let mut buf = Vec::new();
        buf.extend_from_slice(&nregions.to_le_bytes());
        buf.extend_from_slice(&[0u8; 4]); // padding

        for region in borrow.iter() {
            buf.extend_from_slice(&region.mem_offset.to_le_bytes());
            buf.extend_from_slice(&(region.mem_size as u64).to_le_bytes());
            buf.extend_from_slice(&(region.ptr as u64).to_le_bytes());
            buf.extend_from_slice(&0u64.to_le_bytes());
        }

        let ret = unsafe {
            libc::ioctl(
                self.vhost.as_raw_fd(),
                VHOST_SET_MEM_TABLE,
                buf.as_ptr() as *const libc::c_void,
            )
        };
        assert!(ret >= 0, "VHOST_SET_MEM_TABLE failed (errno: {})", unsafe {
            *libc::__errno_location()
        });
    }

    /// Program all vrings on the vhost fd and attach the TAP backend.
    /// Called once from `update()` when both queues are ready.
    fn configure_vhost(&mut self, queues: &mut [VirtioQueue]) {
        if self.configured || queues.len() < 2 {
            return;
        }

        // Both queues must be ready before we configure.
        if !queues[0].ready || !queues[1].ready {
            return;
        }

        // Memory table must be available.
        if self.mem_regions.is_some() {
            self.set_mem_table();
        }

        for i in 0..2 {
            let idx = i as u32;

            // -- vring num --
            let state = vhost_vring_state {
                index: idx,
                num: queues[i].size as u32,
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_NUM,
                    &state as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_SET_VRING_NUM[{}] failed", i);

            // -- vring addr (GPA → HVA) --
            let desc_hva = self
                .gpa_to_hva(queues[i].desc_addr)
                .expect("desc_addr outside mapped regions");
            let avail_hva = self
                .gpa_to_hva(queues[i].avail_addr)
                .expect("avail_addr outside mapped regions");
            let used_hva = self
                .gpa_to_hva(queues[i].used_addr)
                .expect("used_addr outside mapped regions");

            let addr = vhost_vring_addr {
                index: idx,
                flags: 0,
                desc_user_addr: desc_hva,
                used_user_addr: used_hva,
                avail_user_addr: avail_hva,
                log_guest_addr: 0,
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_ADDR,
                    &addr as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_SET_VRING_ADDR[{}] failed", i);

            // -- vring base (start indices = 0) --
            let base = vhost_vring_state { index: idx, num: 0 };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_BASE,
                    &base as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_SET_VRING_BASE[{}] failed", i);

            // -- kick eventfd (userspace→kernel) --
            let kick = Self::make_eventfd();
            let file = vhost_vring_file {
                index: idx,
                fd: kick.as_raw_fd(),
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_KICK,
                    &file as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_SET_VRING_KICK[{}] failed", i);
            self.kick_evt[i] = Some(kick);

            // -- call eventfd (kernel→userspace) --
            let call = Self::make_eventfd();
            let file = vhost_vring_file {
                index: idx,
                fd: call.as_raw_fd(),
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_CALL,
                    &file as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_SET_VRING_CALL[{}] failed", i);
            self.call_evt[i] = Some(call);
        }

        // Attach the TAP backend to both queues (0 = RX, 1 = TX).
        for i in 0..2 {
            let backend = vhost_vring_file {
                index: i as u32,
                fd: self.tap.as_raw_fd(),
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_NET_SET_BACKEND,
                    &backend as *const _ as *const libc::c_void,
                )
            };
            assert!(ret >= 0, "VHOST_NET_SET_BACKEND[{}] failed", i);
        }

        self.configured = true;
    }

    /// Drain call eventfd(s) and check the used ring index directly in guest
    /// memory.  Returns true if vhost has completed new work since the last
    /// time we checked.
    fn check_completions(&mut self, queues: &[VirtioQueue]) -> bool {
        let mut did_work = false;

        for evt in &self.call_evt {
            if let Some(f) = evt {
                let mut buf = [0u8; 8];
                let n = unsafe {
                    libc::read(f.as_raw_fd(), buf.as_mut_ptr() as *mut libc::c_void, 8)
                };
                if n > 0 {
                    did_work = true;
                }
            }
        }

        let Some(ref mem) = self.guest_memory else {
            return did_work;
        };

        for (i, q) in queues.iter().enumerate().take(2) {
            if q.ready && q.used_addr != 0 {
                let used_idx = mem.read_u16(q.used_addr + 2);
                if used_idx != self.last_used_idx[i] {
                    self.last_used_idx[i] = used_idx;
                    did_work = true;
                }
            }
        }

        did_work
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u32 {
        VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.mem_regions = Some(guest_memory.memory_regions());
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        if !self.configured {
            return false;
        }

        Self::kick_vhost(queue_sel, &self.kick_evt[queue_sel]);

        let queues = std::slice::from_ref(queue);
        self.check_completions(queues)
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        self.configure_vhost(queues);
        self.check_completions(queues)
    }
}
