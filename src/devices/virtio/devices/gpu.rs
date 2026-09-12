use crate::devices::virtio::virtio::VirtioDevice;
use crate::devices::virtio::virtio::VirtioGuestMemoryHandle;
use crate::devices::virtio::virtio::VirtioQueue;

pub struct VirtioGpuConfig {}

impl VirtioGpuConfig {
    pub fn new() -> Self {
        Self {}
    }

    pub fn to_bytes(&self, length: usize) -> Vec<u8> {
        vec![]
    }
}

pub struct VirtioGpu {
    guest_mem: Option<VirtioGuestMemoryHandle>,
    config: VirtioGpuConfig,
}

impl VirtioGpu {
    pub fn new() -> Self {
        Self {
            guest_mem: None,
            config: VirtioGpuConfig::new(),
        }
    }
}

impl VirtioDevice for VirtioGpu {
    fn virtio_type(&self) -> u32 {
        16
    }

    fn features(&self) -> u64 {
        0
    }

    fn pass_guest_memory(&mut self, guest_memory: VirtioGuestMemoryHandle) {
        self.guest_mem = Some(guest_memory);
    }

    fn tick(&mut self, _queue_sel: usize, _queue: &mut VirtioQueue) -> bool {
        false
    }

    fn read_config(&self, _length: usize) -> Vec<u8> {
        self.config.to_bytes()
    }

    fn update(&mut self, _queues: &mut [VirtioQueue]) -> bool {
        false
    }
}
