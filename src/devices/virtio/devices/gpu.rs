use crate::devices::virtio::virtio::VirtioDevice;
use crate::devices::virtio::virtio::VirtioGuestMemoryHandle;
use crate::devices::virtio::virtio::VirtioQueue;
use crate::platform::display::DisplayBackend;

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
        if num_scanouts < 16 {
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

pub struct VirtioGpu {
    guest_memory: Option<VirtioGuestMemoryHandle>,
    config: VirtioGpuConfig,
    _window: Box<dyn DisplayBackend>,
}

impl VirtioGpu {
    /// Creates a new vitio GPU device which will provide
    /// the simplest form of video out for the guest
    pub fn new(_window: Box<dyn DisplayBackend>) -> Self {
        Self {
            guest_memory: None,
            config: VirtioGpuConfig::new(0, 1, 0).unwrap(),
            _window,
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
                while let Some(_head) = queue.pop_avail(guest_memory) {}
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
