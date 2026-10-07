use std::sync::{Arc, Mutex};

use kvm_ioctls::VmFd;

use crate::{
    device_maps::{io::IODeviceMap, mmio::MMIODeviceMap},
    machine_config::memory_region::GuestMemoryHandle,
    platform::display::DisplayBackend,
    vcpu::VCPU,
};

pub struct VirtualMachine {
    pub(crate) vcpus: Vec<Arc<Mutex<VCPU>>>,
    pub(crate) vm: Arc<Mutex<VmFd>>,
    pub(crate) io_map: Arc<Mutex<IODeviceMap>>,
    pub(crate) mmio_map: Arc<Mutex<MMIODeviceMap>>,
    pub(crate) memory_regions: GuestMemoryHandle,
    pub(crate) display: Option<Arc<Mutex<Box<dyn DisplayBackend + Send>>>>,
}

impl VirtualMachine {
    pub fn set_display(&mut self, display: Arc<Mutex<Box<dyn DisplayBackend + Send>>>) {
        self.display = Some(display);
    }
}
