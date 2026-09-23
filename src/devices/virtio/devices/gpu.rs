use std::collections::HashMap;
use std::mem;

use crate::devices::virtio::virtio::VIRTQ_DESC_F_NEXT;
use crate::devices::virtio::virtio::VIRTQ_DESC_F_WRITE;
use crate::devices::virtio::virtio::VirtioDevice;
use crate::devices::virtio::virtio::VirtioGuestMemoryHandle;
use crate::devices::virtio::virtio::VirtioQueue;
use crate::platform::display::DisplayBackend;

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

/// Empty responce from a given command
const VIRTIO_GPU_RESP_OK_NODATA: u32 = 0x1100;
/// Device filled up write descirptor with display info
const VIRTIO_GPU_RESP_OK_DISPLAY_INFO: u32 = 0x1101;

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
    pub fn from_bytes(data: &Vec<u8>) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
    }

    /// Creates a stream of bytes from the struct info
    pub fn to_bytes(&self) -> Vec<u8> {
        let size = mem::size_of::<Self>();
        let mut vec = Vec::with_capacity(size);

        unsafe {
            // Cast struct reference to u8 slice
            let bytes = std::slice::from_raw_parts(self as *const Self as *const u8, size);
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
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    enabled: u32,
    flags: u32,
}

impl VirtioGpuDisplayInfo {
    /// Create a new display from a display backend
    pub fn new(backend: &Box<dyn DisplayBackend + Send>) -> Self {
        let (width, height) = backend.get_display_size();

        // Just one display for now
        Self {
            x: 0,
            y: 0,
            width,
            height,
            enabled: 1,
            flags: 0,
        }
    }

    /// Creates an empty display to fill the array
    pub fn empty() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
            enabled: 0,
            flags: 0,
        }
    }

    /// Converts a stream of bytes into this struct
    /// Makes it easy to phase fromn the Virtio Queue
    pub fn to_bytes(&self) -> Vec<u8> {
        let size = mem::size_of::<Self>();
        let mut vec = Vec::with_capacity(size);

        unsafe {
            // Cast struct reference to u8 slice
            let bytes = std::slice::from_raw_parts(self as *const Self as *const u8, size);
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
        let mut display_list: [VirtioGpuDisplayInfo; 16] = [VirtioGpuDisplayInfo::empty(); 16];
        for i in 0..displays.len().min(16) {
            display_list[i] = displays[i];
        }

        Self {
            hdr: VirtioGpuCtrlHdr::new(VIRTIO_GPU_RESP_OK_DISPLAY_INFO),
            display_list,
        }
    }

    pub fn to_bytes(&self) -> Vec<u8> {
        let mut hdr_bytes = self.hdr.to_bytes();
        for display in self.display_list {
            hdr_bytes.extend(display.to_bytes());
        }
        hdr_bytes
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
    pub fn from_bytes(data: &Vec<u8>) -> Option<Self> {
        if data.len() < std::mem::size_of::<Self>() {
            return None;
        }

        let (_, body, _) = unsafe { data.align_to::<Self>() };
        Some(*body.first().expect("Buffer too small"))
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
        let field = offset / 4;
        let start = offset % 4;
        if start + data.len() > 4 && (offset / 4) == ((offset + data.len() - 1) / 4) {
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
}

#[repr(C, packed)]
struct VirtioGpuMemEntry {
    addr: u64,
    length: u32,
    padding: u32,
}

struct GpuResource {
    format: u32,
    width: u32,
    height: u32,
    backing: Vec<VirtioGpuMemEntry>,
}

impl GpuResource {
    /// Create a new GPU Resource, no Backings yet
    pub fn new(format: u32, width: u32, height: u32) -> Self {
        Self {
            format,
            width,
            height,
            backing: vec![],
        }
    }
}

/// Virtio GPU Device
pub struct VirtioGpu {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioGpuConfig,
    window: Box<dyn DisplayBackend + Send>,
    resources: HashMap<u32, GpuResource>,
}

impl VirtioGpu {
    /// Creates a new vitio GPU device which will provide
    /// the simplest form of video out for the guest
    pub fn new(window: Box<dyn DisplayBackend + Send>) -> Self {
        Self {
            guest_memory: None,
            config: VirtioGpuConfig::new(0, 1, 0).unwrap(),
            window,
            resources: HashMap::new(),
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

        let mut _did_work: bool = false;
        match queue_sel {
            0 => {
                // Control Queue
                while let Some(head) = queue.pop_avail(guest_memory) {
                    let header_desc = queue.get_descriptor(guest_memory, head);
                    if header_desc.flags & VIRTQ_DESC_F_NEXT == 0
                        || header_desc.flags & VIRTQ_DESC_F_WRITE != 0
                    {
                        continue;
                    }

                    let mut header_bytes = vec![0; header_desc.len as usize];
                    guest_memory.read_guest_memory(header_desc.addr, &mut header_bytes);

                    let Some(header) = VirtioGpuCtrlHdr::from_bytes(&header_bytes) else {
                        queue.push_used(guest_memory, head, 0);
                        continue;
                    };

                    let gpu_cmd_desc = queue.get_descriptor(guest_memory, header_desc.next); // Response descriptor
                    let written: usize = match header.typ {
                        VIRTIO_GPU_CMD_GET_DISPLAY_INFO => 'get_display_info: {
                            let displays = vec![VirtioGpuDisplayInfo::new(&self.window)];
                            let buf = VirtioGpuDisplayInfoResponse::new(displays).to_bytes();
                            if gpu_cmd_desc.flags & VIRTQ_DESC_F_WRITE == 0
                                || buf.len() > gpu_cmd_desc.len as usize
                            {
                                break 'get_display_info 0;
                            }

                            guest_memory.write_guest_memory(gpu_cmd_desc.addr, buf.as_slice());
                            buf.len()
                        }
                        VIRTIO_GPU_CMD_RESOURCE_CREATE_2D => 'create_resource_2d: {
                            let Some(resource_info) =
                                VirtioGpuResourceCreate2D::from_bytes(&header_bytes)
                            else {
                                break 'create_resource_2d 0;
                            };

                            self.resources.insert(
                                resource_info.resource_id,
                                GpuResource::new(
                                    resource_info.format,
                                    resource_info.width,
                                    resource_info.height,
                                ),
                            );

                            let buf = VirtioGpuCtrlHdr::new(VIRTIO_GPU_RESP_OK_NODATA).to_bytes();
                            if gpu_cmd_desc.flags & VIRTQ_DESC_F_WRITE == 0
                                || buf.len() > gpu_cmd_desc.len as usize
                            {
                                break 'create_resource_2d 0;
                            }
                            guest_memory.write_guest_memory(gpu_cmd_desc.addr, buf.as_slice());
                            buf.len()
                        }
                        VIRTIO_GPU_CMD_RESOURCE_UNREF => 0,
                        VIRTIO_GPU_CMD_SET_SCANOUT => 0,
                        VIRTIO_GPU_CMD_RESOURCE_FLUSH => 0,
                        VIRTIO_GPU_CMD_TRANSFER_TO_HOST_2D => 0,
                        VIRTIO_GPU_CMD_RESOURCE_ATTACH_BACKING => 0,
                        VIRTIO_GPU_CMD_RESOURCE_DETACH_BACKING => 0,
                        VIRTIO_GPU_CMD_GET_EDID => 0,
                        _ => 0,
                    };

                    queue.push_used(guest_memory, head, written as u32);
                }
            }
            1 => {
                // Cursor Queue
                while let Some(_head) = queue.pop_avail(guest_memory) {}
            }
            _ => {}
        }
        _did_work
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
}
