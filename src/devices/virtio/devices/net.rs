use std::{
    fs::File,
    os::fd::{AsRawFd, FromRawFd},
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    devices::virtio::virtio::{IrqCallback, VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue},
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
const VHOST_GET_FEATURES: libc::c_ulong = 0x8008_AF00;

// ---------------------------------------------------------------------------
// Virtio-net constants
// ---------------------------------------------------------------------------
const MAC_ADDRESS: [u8; 6] = [0x52, 0x54, 0x00, 0x12, 0x34, 0x56];
const VIRTIO_NET_F_MAC: u64 = 1 << 5;
const VIRTIO_NET_F_STATUS: u64 = 1 << 16;
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
    configured: bool,
    irq_cb: Option<IrqCallback>,
    listener_handles: Vec<thread::JoinHandle<()>>,
    /// Last-seen used-ring index for each queue, used as a polling fallback
    /// to detect vhost-net completions even if the call-eventfd path stalls.
    last_used_idx: [u16; 2],
    /// Features negotiated with the guest driver.
    driver_features: u64,
    /// Features supported by the vhost-net kernel backend.
    vhost_features: u64,
    /// Set after a permanent vhost setup failure to suppress further retries.
    setup_failed: bool,
}

impl NetVirtio {
    /// Open `/dev/vhost-net`, claim ownership, and query vhost-net's
    /// supported feature set.  A minimal baseline (VIRTIO_F_VERSION_1) is
    /// negotiated upfront so the kernel accepts subsequent vring and backend
    /// ioctls; the full feature set is negotiated later when the guest driver
    /// writes its accepted features via `negotiate_features()`.
    fn open_and_init_vhost() -> (File, u64) {
        let vhost = Self::open_vhost();
        unsafe {
            let ret = libc::ioctl(vhost.as_raw_fd(), VHOST_SET_OWNER);
            if ret < 0 {
                panic!(
                    "VHOST_SET_OWNER failed (errno: {})",
                    *libc::__errno_location()
                );
            }
        }
        let mut features: u64 = 0;
        let ret = unsafe {
            libc::ioctl(
                vhost.as_raw_fd(),
                VHOST_GET_FEATURES,
                &mut features as *mut _ as *mut libc::c_void,
            )
        };
        if ret < 0 {
            panic!(
                "VHOST_GET_FEATURES failed (errno: {})",
                unsafe { *libc::__errno_location() }
            );
        }
        // Set only VIRTIO_F_VERSION_1 as a baseline so the kernel allows
        // subsequent vring and backend ioctls.  The full guest-negotiated
        // feature set is applied later in `negotiate_features()`.
        const VHOST_SET_FEATURES: libc::c_ulong = 0x4008_AF00;
        let baseline: u64 = 1u64 << 32;
        let ret = unsafe {
            libc::ioctl(
                vhost.as_raw_fd(),
                VHOST_SET_FEATURES,
                &baseline as *const _ as *const libc::c_void,
            )
        };
        if ret < 0 {
            panic!(
                "VHOST_SET_FEATURES failed (errno: {})",
                unsafe { *libc::__errno_location() }
            );
        }
        (vhost, features)
    }

    pub fn new(tap_name: &str) -> Self {
        let tap = Self::open_tap(tap_name);
        let (vhost, vhost_features) = Self::open_and_init_vhost();

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
            configured: false,
            irq_cb: None,
            listener_handles: Vec::new(),
            last_used_idx: [0, 0],
            driver_features: 0,
            vhost_features,
            setup_failed: false,
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

        let ret = unsafe { libc::ioctl(fd, TUNSETIFF, &ifr as *const _ as *const libc::c_void) };
        assert!(
            ret >= 0,
            "TUNSETIFF failed for '{}' (errno: {}, run scripts/enable_tap.sh first, \
             or use cargo-run.sh for cap_net_admin)",
            name,
            unsafe { *libc::__errno_location() },
        );

        // Bump the send buffer so vhost-net's sendmsg doesn't hit
        // EAGAIN on tun_alloc_skb (sk_wmem_alloc >= sk_sndbuf).
        // Default is ~212KiB (net.core.wmem_default); max is usually 4MiB.
        // NOTE: setsockopt(SO_SNDBUF) does not work on a TAP fd because
        // the fd is a character device, not a socket.  Use TUNSETSNDBUF
        // ioctl instead.
        const TUNSETSNDBUF: libc::c_ulong = 0x4004_54D4;
        let bufsize: libc::c_int = 4 * 1024 * 1024; // kernel doubles this
        let ret = unsafe {
            libc::ioctl(
                fd,
                TUNSETSNDBUF,
                &bufsize as *const _ as *const libc::c_void,
            )
        };
        if ret < 0 {
            let errno = unsafe { *libc::__errno_location() };
            eprintln!("TUNSETSNDBUF(4MiB) failed (errno: {})", errno);
        }

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

    /// `blocking = false` for the kick eventfd (we only ever write to it).
    /// `blocking = true` for the call eventfd (the listener thread blocks
    /// in `read()` until vhost-net signals it — no busy-polling, and no
    /// dependency on the guest ever causing another vmexit).
    fn make_eventfd(blocking: bool) -> File {
        let flags = if blocking { 0 } else { libc::EFD_NONBLOCK };
        let fd = unsafe { libc::eventfd(0, flags) };
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
    /// processes the given virtqueue immediately rather than waiting for its
    /// own internal polling interval.
    fn kick_vhost(evt: &Option<File>) {
        if let Some(f) = evt {
            let val = 1u64;
            unsafe {
                libc::write(f.as_raw_fd(), &val as *const _ as *const libc::c_void, 8);
            }
        }
    }

    // -- vhost setup -------------------------------------------------------

    /// Share the guest memory layout with the vhost kernel subsystem.
    /// Returns `true` on success.
    fn set_mem_table(&self) -> bool {
        let Some(ref regions) = self.mem_regions else {
            return false;
        };
        let borrow = match regions.lock() {
            Ok(g) => g,
            Err(_) => return false,
        };
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
        drop(borrow);

        let ret = unsafe {
            libc::ioctl(
                self.vhost.as_raw_fd(),
                VHOST_SET_MEM_TABLE,
                buf.as_ptr() as *const libc::c_void,
            )
        };
        if ret < 0 {
            let errno = unsafe { *libc::__errno_location() };
            eprintln!("VHOST_SET_MEM_TABLE failed (errno: {})", errno);
        }
        ret >= 0
    }

    /// Program all vrings on the vhost fd, attach the TAP backend, and spawn
    /// a completion-listener thread per queue. Called once from `update()`
    /// when both queues are ready.
    fn configure_vhost(&mut self, queues: &mut [VirtioQueue]) {
        if self.configured || self.setup_failed || queues.len() < 2 {
            return;
        }

        if !queues[0].ready || !queues[1].ready {
            return;
        }

        if self.mem_regions.is_some() && !self.set_mem_table() {
            eprintln!("vhost-net: set_mem_table failed, skipping vhost setup");
            self.setup_failed = true;
            return;
        }

        let mut call_evts: [Option<File>; 2] = [None, None];

        // Set up vrings AND attach the backend for each queue individually,
        // before touching the next queue.  Some kernels seem to invalidate
        // queue-0's state when queue-1's vring is configured, so attaching
        // the backend early works around that.
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
            if ret < 0 {
                eprintln!("vhost-net: VHOST_SET_VRING_NUM[{}] failed", i);
                self.setup_failed = true;
                return;
            }

            // -- vring addr (GPA → HVA) --
            let desc_hva = match self.gpa_to_hva(queues[i].desc_addr) {
                Some(hva) => hva,
                None => {
                    eprintln!("vhost-net: desc_addr outside mapped regions");
                    self.setup_failed = true;
                    return;
                }
            };
            let avail_hva = match self.gpa_to_hva(queues[i].avail_addr) {
                Some(hva) => hva,
                None => {
                    eprintln!("vhost-net: avail_addr outside mapped regions");
                    self.setup_failed = true;
                    return;
                }
            };
            let used_hva = match self.gpa_to_hva(queues[i].used_addr) {
                Some(hva) => hva,
                None => {
                    eprintln!("vhost-net: used_addr outside mapped regions");
                    self.setup_failed = true;
                    return;
                }
            };

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
            if ret < 0 {
                let errno = unsafe { *libc::__errno_location() };
                eprintln!(
                    "vhost-net: VHOST_SET_VRING_ADDR[{}] failed (errno: {})",
                    i, errno
                );
                self.setup_failed = true;
                return;
            }

            // -- vring base (start indices = 0) --
            let base = vhost_vring_state { index: idx, num: 0 };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_SET_VRING_BASE,
                    &base as *const _ as *const libc::c_void,
                )
            };
            if ret < 0 {
                eprintln!("vhost-net: VHOST_SET_VRING_BASE[{}] failed", i);
                self.setup_failed = true;
                return;
            }

            // -- kick eventfd (userspace→kernel), nonblocking, we only write --
            let kick = Self::make_eventfd(false);
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
            if ret < 0 {
                eprintln!("vhost-net: VHOST_SET_VRING_KICK[{}] failed", i);
                self.setup_failed = true;
                return;
            }
            self.kick_evt[i] = Some(kick);

            // -- call eventfd (kernel→userspace), blocking, read by a thread --
            let call = Self::make_eventfd(true);
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
            if ret < 0 {
                eprintln!("vhost-net: VHOST_SET_VRING_CALL[{}] failed", i);
                self.setup_failed = true;
                return;
            }
            call_evts[i] = Some(call);

            // -- TAP backend (must be attached before moving on to the next
            //    queue because some kernel versions invalidate the vring
            //    state of queue-0 when queue-1 is configured).
            let backend = vhost_vring_file {
                index: idx,
                fd: self.tap.as_raw_fd(),
            };
            let ret = unsafe {
                libc::ioctl(
                    self.vhost.as_raw_fd(),
                    VHOST_NET_SET_BACKEND,
                    &backend as *const _ as *const libc::c_void,
                )
            };
            if ret < 0 {
                let errno = unsafe { *libc::__errno_location() };
                eprintln!(
                    "vhost-net: VHOST_NET_SET_BACKEND[{}] failed (errno: {})",
                    i, errno
                );
                self.setup_failed = true;
                return;
            }
        }

        for i in 0..2 {
            let call_evt = call_evts[i].take().expect("call evt must be set");
            let irq_cb = self.irq_cb.clone();

            let handle = thread::Builder::new()
                .name(format!("vhost-net-call-{}", i))
                .spawn(move || {
                    let mut buf = [0u8; 8];
                    loop {
                        let n = unsafe {
                            libc::read(
                                call_evt.as_raw_fd(),
                                buf.as_mut_ptr() as *mut libc::c_void,
                                8,
                            )
                        };
                        if n < 0 {
                            let errno = unsafe { *libc::__errno_location() };
                            if errno == libc::EINTR {
                                continue;
                            }
                            // fd closed/torn down — stop the thread.
                            break;
                        }

                        if let Some(cb) = &irq_cb {
                            cb();
                        }
                    }
                })
                .expect("failed to spawn vhost-net completion listener");

            self.listener_handles.push(handle);
        }

        self.configured = true;
    }
}

impl VirtioDevice for NetVirtio {
    fn virtio_type(&self) -> u32 {
        0x01
    }

    fn features(&self) -> u64 {
        VIRTIO_NET_F_MAC | VIRTIO_NET_F_STATUS
    }

    fn negotiate_features(&mut self, driver_features: u64) {
        self.driver_features = driver_features;
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.mem_regions = Some(guest_memory.memory_regions());
        self.guest_memory = Some(guest_memory);
    }

    fn set_irq_callback(&mut self, cb: IrqCallback) {
        self.irq_cb = Some(cb);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        if !self.configured || queue_sel > 1 {
            return false;
        }

        // Forward the guest's notification into the kernel so vhost-net
        // processes the queue immediately.
        Self::kick_vhost(&self.kick_evt[queue_sel]);

        // Poll the used ring as a fallback: even if the listener-thread /
        // call-eventfd path stalls, the tick thread picks up completions
        // within 100 us.
        if let Some(ref mem) = self.guest_memory {
            if queue.ready && queue.used_addr != 0 {
                let used_idx = mem.read_u16(queue.used_addr + 2);
                if used_idx != self.last_used_idx[queue_sel] {
                    self.last_used_idx[queue_sel] = used_idx;
                    return true;
                }
            }
        }

        false
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn update(&mut self, queues: &mut [VirtioQueue]) -> bool {
        self.configure_vhost(queues);
        false
    }

    fn reset(&mut self) {
        self.configured = false;
        // Open a fresh vhost fd — the old one still points to the previous
        // queue addresses and is effectively stale after a guest-driven reset.
        let (vhost, vhost_features) = Self::open_and_init_vhost();
        self.vhost = vhost;
        self.vhost_features = vhost_features;
        self.driver_features = 0;
        self.setup_failed = false;
        self.kick_evt = [None, None];
        self.last_used_idx = [0, 0];
        // Old listener threads still block on the previous call eventfds but
        // will never fire (no signals arrive on disconnected fds).  They are
        // harmless background threads that will be cleaned up on process exit.
        self.listener_handles.clear();
    }
}
