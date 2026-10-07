use std::collections::HashMap;
use std::mem;
use std::sync::{Arc, Mutex};

use crate::devices::virtio::virtio::VIRTQ_DESC_F_NEXT;
use crate::devices::virtio::virtio::VIRTQ_DESC_F_WRITE;
use crate::devices::virtio::virtio::VirtioDevice;
use crate::devices::virtio::virtio::VirtioGuestMemoryHandle;
use crate::devices::virtio::virtio::VirtioQueue;
use crate::devices::virtio::virtio::VirtqDesc;
use crate::platform::display::DisplayBackend;
use crate::platform::display::DisplayRect;

const MAX_SCANOUTS: usize = 16;
const MAX_REQUEST_BYTES: usize = 4096;

const MAX_RESOURCE_BYTES: usize = 256 * 1024 * 1024;
const MAX_RESOURCE_DIMENSION: u32 = 16384;
const MAX_RESOURCES: usize = 64;

const BYTES_PER_PIXEL: usize = 4;

/// Query display capabilities
const VIRTIO_GPU_CMD_GET_DISPLAY_INFO: u32 = 0x0100;
/// Create 2D rendering resources
const VIRTIO_GPU_CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
/// Release GPU resources
const VIRTIO_GPU_CMD_RESOURCE_UNREF: u32 = 0x0102;
/// Configure display output
const VIRTIO_GPU_CMD_SET_SCANOUT: u32 = 0x0103;
/// Make rendering visible
const VIRTIO_GPU_CMD_RESOURCE_FLUSH: u32 = 0x0104;
/// Transfer framebuffer to GPU
const VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
/// Attach memory to resources
const VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
/// Detach memory from resources
const VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING: u32 = 0x0107;
/// Retrieve monitor EDID data
const VIRTIO_GPU_CMD_GET_EDID: u32 = 0x010A;
/// Move the cursor, sent on the cursor queue
const VIRTIO_GPU_CMD_UPDATE_CURSOR: u32 = 0x010B;
/// Query the current cursor
const VIRTIO_GPU_CMD_CURSOR_GET: u32 = 0x010C;

/// Empty responce from a given command
const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
/// Device filled up write descirptor with display info
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;
/// Device does not support the requested command
const VIRTIO_GPU_RESP_ERR_UNSUPPORTED: u32 = 0x1201;
/// A field in the request was out of range
const VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT: u32 = 0x1202;
const VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID: u32 = 0x1203;

const SUPPORTED_FORMATS: [u32; 4] = [
    1, // VIRTIO_GPU_FORMAT_B8G8R8A8_UNORM
    2, // VIRTIO_GPU_FORMAT_B8G8R8X8_UNORM
    3, // VIRTIO_GPU_FORMAT_A8B8G8R8_UNORM
    4, // VIRTIO_GPU_FORMAT_X8B8G8R8_UNORM
];

fn write_response(guest_memory: &mut VirtioGuestMemoryHandle, desc: &VirtqDesc, typ: u32) -> usize {
    let buf = VirtioGpuCtrlHdr::new(typ).to_bytes();
    if (desc.flags & VIRTQ_DESC_F_WRITE) == 0 || buf.len() > desc.len as usize {
        return 0;
    }

    guest_memory.write_guest_memory(desc.addr, buf.as_slice());
    buf.len()
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuMemEntry {
    addr: u64,
    length: u32,
    padding: u32,
}

impl VirtioGpuMemEntry {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

/// The Header for all GPU commands
/// use `from_bytes()` to get the current instance from a bytes vector
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuCtrlHdr {
    typ: u32,
    flags: u32,
    fence_id: u64,
    ctx_id: u32,
    ring_idx: u32,
}

impl VirtioGpuCtrlHdr {
    /// Creates a new version of the struct as simply as possible
    pub fn new(typ: u32) -> Self {
        Self {
            typ,
            flags: 0,
            fence_id: 0,
            ctx_id: 0,
            ring_idx: 0,
        }
    }

    /// Converts a stream of bytes into this struct
    /// Makes it easy to phase from the Virtio Queue
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }

    /// Creates a stream of bytes from the struct info
    pub fn to_bytes(self) -> Vec<u8> {
        let size = mem::size_of::<Self>();
        let mut vec = Vec::with_capacity(size);

        unsafe {
            // Cast struct reference to u8 slice
            let bytes = std::slice::from_raw_parts(&self as *const Self as *const u8, size);
            vec.extend_from_slice(bytes);
        }

        vec
    }
}

/// Info about one display on the GPU, can have multipule
/// on every device
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuDisplayInfo {
    rect: DisplayRect,
    enabled: u32,
    flags: u32,
}

impl VirtioGpuDisplayInfo {
    /// Create a new display from a display backend
    pub fn new(backend: &dyn DisplayBackend) -> Self {
        let (width, height) = backend.get_display_size();

        let rect = DisplayRect {
            x: 0,
            y: 0,
            width,
            height,
        };

        Self {
            rect,
            enabled: 1,
            flags: 0,
        }
    }

    /// Creates an empty display to fill the array
    pub fn empty() -> Self {
        let rect = DisplayRect {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        };

        Self {
            rect,
            enabled: 0,
            flags: 0,
        }
    }

    /// Converts a stream of bytes into this struct
    /// Makes it easy to phase fromn the Virtio Queue
    pub fn to_bytes(self) -> Vec<u8> {
        let size = mem::size_of::<Self>();
        let mut vec = Vec::with_capacity(size);

        unsafe {
            // Cast struct reference to u8 slice
            let bytes = std::slice::from_raw_parts(&self as *const Self as *const u8, size);
            vec.extend_from_slice(bytes);
        }

        vec
    }
}

struct VirtioGpuDisplayInfoResponse {
    hdr: VirtioGpuCtrlHdr,
    display_list: [VirtioGpuDisplayInfo; 16],
}

impl VirtioGpuDisplayInfoResponse {
    pub fn new(displays: Vec<VirtioGpuDisplayInfo>) -> Self {
        let count = displays.len().min(16);
        let mut display_list: [VirtioGpuDisplayInfo; 16] = [VirtioGpuDisplayInfo::empty(); 16];
        display_list[..count].copy_from_slice(&displays[..count]);

        Self {
            hdr: VirtioGpuCtrlHdr::new(VIRTIO_GPU_RESP_OK_DISPLAY_INFO),
            display_list,
        }
    }

    pub fn to_bytes(self) -> Vec<u8> {
        let mut bytes = self.hdr.to_bytes();
        for display in self.display_list.iter() {
            bytes.extend(display.to_bytes());
        }
        bytes
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuResourceCreate2D {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
    format: u32,
    width: u32,
    height: u32,
}

impl VirtioGpuResourceCreate2D {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuResourceDrop {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
}

impl VirtioGpuResourceDrop {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuSetScanout {
    hdr: VirtioGpuCtrlHdr,
    rect: DisplayRect,
    scanout_id: u32,
    resource_id: u32,
}

impl VirtioGpuSetScanout {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuTransferToHost {
    hdr: VirtioGpuCtrlHdr,
    rect: DisplayRect,
    offset: u64,
    resource_id: u32,
}

impl VirtioGpuTransferToHost {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuResourceFlush {
    hdr: VirtioGpuCtrlHdr,
    rect: DisplayRect,
    resource_id: u32,
}

impl VirtioGpuResourceFlush {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }
}

#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuBackingAttachHdr {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
    pub nr_entries: u32,
}

impl VirtioGpuBackingAttachHdr {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first()?)
    }
}

struct VirtioGpuBackingAttach {
    hdr: VirtioGpuBackingAttachHdr,
    mem_entries: Vec<VirtioGpuMemEntry>,
}
impl VirtioGpuBackingAttach {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        let hdr = VirtioGpuBackingAttachHdr::from_bytes(data)?;
        let rest = &data[std::mem::size_of::<VirtioGpuBackingAttachHdr>()..];

        let mem_entries_cnt =
            (hdr.nr_entries as usize).min(rest.len() / std::mem::size_of::<VirtioGpuMemEntry>());

        let mut mem_entries: Vec<VirtioGpuMemEntry> = Vec::with_capacity(mem_entries_cnt);
        for mem_entry_indx in 0..mem_entries_cnt {
            let start = mem_entry_indx * std::mem::size_of::<VirtioGpuMemEntry>();
            let end = start + std::mem::size_of::<VirtioGpuMemEntry>();
            mem_entries.push(VirtioGpuMemEntry::from_bytes(&rest[start..end])?);
        }

        Some(Self { hdr, mem_entries })
    }
}

/// `RESOURCE_DETACH_BACKING` carries the header and a bare resource id, with
/// no entry count.
#[repr(C, packed)]
#[derive(Debug, Copy, Clone)]
struct VirtioGpuResourceDetachBacking {
    hdr: VirtioGpuCtrlHdr,
    resource_id: u32,
}

impl VirtioGpuResourceDetachBacking {
    pub fn from_bytes(data: &[u8]) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first()?)
    }
}

/// Config for this device, for gpu it has writable elements to make sure to define
/// it as mutable when using it.
pub struct VirtioGpuConfig {
    events_read: u32,
    events_clear: u32,
    num_scanouts: u32,
    num_capsets: u32,
}

impl VirtioGpuConfig {
    /// Create a new Virtio GPU Config
    /// enforces the limits defined in the spec (https://docs.oasis-open.org/virtio/virtio/v1.3/csd01/virtio-v1.3-csd01.html#x1-3960007)
    pub fn new(events_clear: u32, num_scanouts: u32, num_capsets: u32) -> Option<Self> {
        if num_scanouts == 0 || num_scanouts > 16 {
            return None;
        }

        Some(Self {
            events_read: 0,
            events_clear,
            num_scanouts,
            num_capsets,
        })
    }

    /// Converts the GPU Config into a bytes array
    /// This can then be directly passed to the guest
    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        let mut buf = self.events_read.to_le_bytes().to_vec();
        buf.extend(self.events_clear.to_le_bytes().to_vec());
        buf.extend(self.num_scanouts.to_le_bytes().to_vec());
        buf.extend(self.num_capsets.to_le_bytes().to_vec());

        buf.resize(length, 0);
        buf
    }

    pub fn write_field(&mut self, offset: usize, data: &[u8]) {
        if data.is_empty() || data.len() > 4 {
            // A single MMIO write can straddle the two 32-bit fields, or be
            // wide enough to overflow the mask shift below.
            return;
        }

        let field = offset / 4;
        let start = offset % 4;
        if start + data.len() > 4 {
            return;
        }

        let mask = (!0u32 << (start * 8)) & (!0u32 >> (32 - (start + data.len()) * 8));
        let mut buf = [0u8; 4];
        buf[start..start + data.len()].copy_from_slice(data);

        let cur = match field {
            1 => self.events_clear, // the driver->device field
            _ => return,
        };
        self.events_clear = (cur & !mask) | (u32::from_le_bytes(buf) & mask);
    }

    /// Clears the driver-writable event bits after a device reset.
    pub fn reset(&mut self) {
        self.events_read = 0;
        self.events_clear = 0;
    }
}

struct GpuResource {
    width: u32,
    height: u32,
    backing: Vec<VirtioGpuMemEntry>,
    data: Vec<u8>,
    stride: usize,
}

impl GpuResource {
    /// Create a new GPU Resource with no backing pages attached yet.
    ///
    /// Returns `None` if the requested geometry is unsupported or would need
    /// more host memory than [`MAX_RESOURCE_BYTES`]. Dimensions come straight
    /// from the guest, so this check is what stops a `RESOURCE_CREATE_2D` from
    /// aborting the whole VMM via a failed allocation.
    pub fn new(format: u32, width: u32, height: u32) -> Option<Self> {
        if !SUPPORTED_FORMATS.contains(&format) {
            return None;
        }

        if width == 0 || height == 0 {
            return None;
        }

        if width > MAX_RESOURCE_DIMENSION || height > MAX_RESOURCE_DIMENSION {
            return None;
        }

        let stride = (width as usize).checked_mul(BYTES_PER_PIXEL)?;
        let len = stride.checked_mul(height as usize)?;
        if len > MAX_RESOURCE_BYTES {
            return None;
        }

        Some(Self {
            width,
            height,
            backing: vec![],
            data: vec![0u8; len],
            stride,
        })
    }

    /// True if `rect` lies entirely inside this resource.
    fn contains(&self, rect: &DisplayRect) -> bool {
        rect.width != 0
            && rect.height != 0
            && rect.x < self.width
            && rect.y < self.height
            && rect.x.saturating_add(rect.width) <= self.width
            && rect.y.saturating_add(rect.height) <= self.height
    }

    /// Total size of the attached backing pages.
    fn backing_len(&self) -> usize {
        self.backing
            .iter()
            .fold(0usize, |acc, e| acc.saturating_add(e.length as usize))
    }
}

/// Copies `out.len()` bytes of a resource's backing pages, starting `start`
/// bytes into the logical page list, into `out`.
///
/// The scratch buffer for the guest read is allocated once by the caller, so a
/// full-frame transfer is one allocation per row rather than two.
fn read_backing_range(
    guest_memory: &VirtioGuestMemoryHandle,
    resource: &GpuResource,
    start: usize,
    out: &mut [u8],
) {
    let mut copied = 0usize;
    let mut skipped = 0usize;
    for entry in &resource.backing {
        if copied == out.len() {
            break;
        }
        let entry_len = entry.length as usize;
        let entry_start = start.saturating_sub(skipped);
        if entry_start < entry_len {
            let n = (entry_len - entry_start).min(out.len() - copied);
            let Some(addr) = entry.addr.checked_add(entry_start as u64) else {
                break;
            };
            let tmp = guest_memory.read_guest_memory_alloc(addr, n);
            out[copied..copied + n].copy_from_slice(&tmp);
            copied += n;
        }
        skipped = skipped.saturating_add(entry_len);
    }
}

/// A command's pending reply, written to the response descriptor once the
/// command has been handled.
enum GpuResponse {
    /// A bare `virtio_gpu_ctrl_hdr` carrying the given type code.
    Header(u32),
    /// A pre-rendered response body.
    Raw(Vec<u8>),
}

#[derive(Debug, Copy, Clone)]
struct VirtioGpuScanout {
    resource_id: Option<u32>,
    rect: DisplayRect,
}

impl VirtioGpuScanout {
    fn empty() -> Self {
        Self {
            resource_id: None,
            rect: DisplayRect::empty(),
        }
    }
}

/// Virtio GPU Device
pub struct VirtioGpu {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioGpuConfig,
    window: Arc<Mutex<Box<dyn DisplayBackend + Send>>>,
    resources: HashMap<u32, GpuResource>,
    scanouts: [VirtioGpuScanout; MAX_SCANOUTS],
}

impl VirtioGpu {
    /// Creates a new vitio GPU device which will provide
    /// the simplest form of video out for the guest
    pub fn new(window: Arc<Mutex<Box<dyn DisplayBackend + Send>>>) -> Self {
        Self {
            guest_memory: None,
            config: VirtioGpuConfig::new(0, 1, 0).unwrap(),
            window,
            resources: HashMap::new(),
            scanouts: [VirtioGpuScanout::empty(); MAX_SCANOUTS],
        }
    }
}

impl VirtioDevice for VirtioGpu {
    fn virtio_type(&self) -> u32 {
        0x10
    }

    fn features(&self) -> u64 {
        0x0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_memory = Some(guest_memory);
    }

    fn tick(&mut self, queue_sel: usize, queue: &mut VirtioQueue) -> bool {
        let Some(guest_memory) = self.guest_memory.as_mut() else {
            return false;
        };

        let mut did_work = false;
        match queue_sel {
            0 => {
                // Control Queue
                while let Some(head) = queue.pop_avail(guest_memory) {
                    did_work = true;

                    let header_desc = queue.get_descriptor(guest_memory, head);

                    // The head has to be device-readable, has to chain to a
                    // valid response descriptor, and its length is guest
                    // controlled. Retire anything malformed rather than
                    // `continue`ing, or the avail and used rings drift apart
                    // and the driver hangs.
                    if (header_desc.flags & VIRTQ_DESC_F_NEXT) == 0
                        || (header_desc.flags & VIRTQ_DESC_F_WRITE) != 0
                        || header_desc.next as usize >= queue.size as usize
                        || header_desc.len as usize > MAX_REQUEST_BYTES
                    {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }

                    let header_bytes = guest_memory
                        .read_guest_memory_alloc(header_desc.addr, header_desc.len as usize);

                    let Some(header) = VirtioGpuCtrlHdr::from_bytes(&header_bytes) else {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    };

                    let gpu_cmd_desc = queue.get_descriptor(guest_memory, header_desc.next); // Response descriptor

                    let response: GpuResponse = 'cmd: {
                        match header.typ {
                            VIRTIO_GPU_CMD_GET_DISPLAY_INFO => {
                                let display = self.window.lock().unwrap();
                                let displays =
                                    vec![VirtioGpuDisplayInfo::new(&**display)];
                                GpuResponse::Raw(
                                    VirtioGpuDisplayInfoResponse::new(displays).to_bytes(),
                                )
                            }
                            VIRTIO_GPU_CMD_RESOURCE_CREATE_2D => {
                                let Some(req) =
                                    VirtioGpuResourceCreate2D::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                // Re-using a live id would orphan the previous
                                // resource's backing pages.
                                let resource_id = req.resource_id;
                                if self.resources.contains_key(&resource_id) {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                }

                                if self.resources.len() >= MAX_RESOURCES {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                }

                                let Some(resource) =
                                    GpuResource::new(req.format, req.width, req.height)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                self.resources.insert(resource_id, resource);
                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_RESOURCE_UNREF => {
                                let Some(req) = VirtioGpuResourceDrop::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                let id = req.resource_id;
                                self.resources.remove(&id);

                                for scanout in self.scanouts.iter_mut() {
                                    if scanout.resource_id == Some(id) {
                                        *scanout = VirtioGpuScanout::empty();
                                    }
                                }

                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_SET_SCANOUT => {
                                let Some(req) = VirtioGpuSetScanout::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                if (req.scanout_id as usize) >= MAX_SCANOUTS {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                }

                                let resource_id = req.resource_id;
                                if resource_id != 0 && !self.resources.contains_key(&resource_id) {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                }

                                let scanout = &mut self.scanouts[req.scanout_id as usize];
                                if req.resource_id == 0 {
                                    *scanout = VirtioGpuScanout::empty();
                                } else {
                                    *scanout = VirtioGpuScanout {
                                        resource_id: Some(req.resource_id),
                                        rect: req.rect,
                                    };
                                }

                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_RESOURCE_FLUSH => {
                                let Some(req) = VirtioGpuResourceFlush::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                let resource_id = req.resource_id;
                                let Some(resource) = self.resources.get(&resource_id) else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                };

                                // Note: this blits the whole scanout rather than
                                // just `req.rect`, the dirty region.
                                for scanout in self.scanouts.iter() {
                                    if scanout.resource_id == Some(req.resource_id) {
                                        self.window.lock().unwrap().blit(
                                            &resource.data,
                                            resource.stride,
                                            scanout.rect,
                                        );
                                    }
                                }

                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D => {
                                let Some(req) = VirtioGpuTransferToHost::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                let resource_id = req.resource_id;
                                let Some(resource) = self.resources.get_mut(&resource_id) else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                };

                                let rect = req.rect;
                                if !resource.contains(&rect) {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                }

                                let stride = resource.stride;
                                let row_len = rect.width as usize * BYTES_PER_PIXEL;

                                // `req.offset` is an unchecked guest u64, so every
                                // step of this address arithmetic is checked.
                                let base = match (req.offset as usize)
                                    .checked_add((rect.y as usize).saturating_mul(stride))
                                    .and_then(|v| v.checked_add(rect.x as usize * BYTES_PER_PIXEL))
                                {
                                    Some(base) => base,
                                    None => {
                                        break 'cmd GpuResponse::Header(
                                            VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                        );
                                    }
                                };

                                if base > resource.backing_len() {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                }

                                let mut row_buf = vec![0u8; row_len];
                                for row in 0..rect.height as usize {
                                    let Some(src) = base.checked_add(row.saturating_mul(stride))
                                    else {
                                        break;
                                    };
                                    read_backing_range(guest_memory, resource, src, &mut row_buf);

                                    let dst_start = (rect.y as usize + row) * stride
                                        + rect.x as usize * BYTES_PER_PIXEL;
                                    let copy_len = row_buf
                                        .len()
                                        .min(resource.data.len().saturating_sub(dst_start));
                                    if copy_len == 0 {
                                        break;
                                    }
                                    resource.data[dst_start..dst_start + copy_len]
                                        .copy_from_slice(&row_buf[..copy_len]);
                                }

                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING => {
                                let Some(req) = VirtioGpuBackingAttach::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                let resource_id = req.hdr.resource_id;
                                let Some(resource) = self.resources.get_mut(&resource_id) else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                };

                                resource.backing = req.mem_entries;
                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING => {
                                // Not the attach header: this command is 28
                                // bytes, since it has no nr_entries field.
                                let Some(req) =
                                    VirtioGpuResourceDetachBacking::from_bytes(&header_bytes)
                                else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_ARGUMENT,
                                    );
                                };

                                let resource_id = req.resource_id;
                                let Some(resource) = self.resources.get_mut(&resource_id) else {
                                    break 'cmd GpuResponse::Header(
                                        VIRTIO_GPU_RESP_ERR_INVALID_RESOURCE_ID,
                                    );
                                };

                                resource.backing = vec![];
                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            // No EDID is exposed, but a driver blocks on the
                            // response. Sending one turns a 5s probe timeout
                            // into an instant "no EDID available".
                            VIRTIO_GPU_CMD_GET_EDID => {
                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            VIRTIO_GPU_CMD_CURSOR_GET => {
                                GpuResponse::Header(VIRTIO_GPU_RESP_OK_NODATA)
                            }
                            // Always answer, even for commands we do not
                            // implement: silence makes the driver wait out its
                            // full response timeout.
                            _ => GpuResponse::Header(VIRTIO_GPU_RESP_ERR_UNSUPPORTED),
                        }
                    };

                    let written: usize = match response {
                        GpuResponse::Raw(buf) => {
                            if (gpu_cmd_desc.flags & VIRTQ_DESC_F_WRITE) == 0
                                || buf.len() > gpu_cmd_desc.len as usize
                            {
                                0
                            } else {
                                guest_memory.write_guest_memory(gpu_cmd_desc.addr, buf.as_slice());
                                buf.len()
                            }
                        }
                        GpuResponse::Header(typ) => {
                            write_response(guest_memory, &gpu_cmd_desc, typ)
                        }
                    };

                    queue.push_used(guest_memory, head, written as u32);
                }
            }
            1 => {
                // Cursor Queue. Still retire the descriptors and reply, or the
                // driver's cursor updates wedge the queue.
                while let Some(head) = queue.pop_avail(guest_memory) {
                    did_work = true;

                    let header_desc = queue.get_descriptor(guest_memory, head);
                    if (header_desc.flags & VIRTQ_DESC_F_NEXT) == 0
                        || (header_desc.flags & VIRTQ_DESC_F_WRITE) != 0
                        || header_desc.next as usize >= queue.size as usize
                    {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    }

                    let resp_desc = queue.get_descriptor(guest_memory, header_desc.next);
                    let header_len = (header_desc.len as usize).min(MAX_REQUEST_BYTES);
                    let header_bytes =
                        guest_memory.read_guest_memory_alloc(header_desc.addr, header_len);
                    let typ =
                        VirtioGpuCtrlHdr::from_bytes(&header_bytes).map_or(0, |header| header.typ);

                    // The reply is a response header, never an echo of the
                    // command code.
                    let response = match typ {
                        VIRTIO_GPU_CMD_UPDATE_CURSOR | VIRTIO_GPU_CMD_CURSOR_GET => {
                            VIRTIO_GPU_RESP_OK_NODATA
                        }
                        _ => VIRTIO_GPU_RESP_ERR_UNSUPPORTED,
                    };

                    let written = write_response(guest_memory, &resp_desc, response);
                    queue.push_used(guest_memory, head, written as u32);
                }
            }
            _ => {}
        }

        did_work
    }

    fn read_config(&self, length: usize) -> Vec<u8> {
        self.config.to_bytes(length)
    }

    fn write_config(&mut self, offset: usize, data: &[u8]) {
        self.config.write_field(offset, data);
    }

    fn update(&mut self, _queues: &mut [VirtioQueue]) -> bool {
        false
    }

    /// A guest reset must not leave the old resource table and scanout
    /// bindings pointing at guest pages that are no longer ours.
    fn reset(&mut self) {
        self.resources.clear();
        self.scanouts = [VirtioGpuScanout::empty(); MAX_SCANOUTS];
        self.config.reset();
    }
}
