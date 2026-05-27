use std::sync::{Arc, Mutex};

use kvm_ioctls::VmFd;

use crate::{device_maps::{io::IODeviceMap, mmio::MMIODeviceMap}, memory_region::GuestMemoryHandle, vcpu::VCPU};

pub struct VirtualMachine {
    pub(crate) vcpu: VCPU,
    pub(crate) vm: Arc<Mutex<VmFd>>,
    pub(crate) io_map: Arc<Mutex<IODeviceMap>>,
    pub(crate) mmio_map: Arc<Mutex<MMIODeviceMap>>,
    pub(crate) memory_regions: GuestMemoryHandle,
}