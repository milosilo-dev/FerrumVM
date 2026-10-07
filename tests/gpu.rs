//! Tests for the virtio-GPU device.
//!
//! The device is driven the way a real driver drives it: requests are laid
//! out in guest memory as raw wire-format bytes, pointed at by a virtqueue
//! descriptor pair, and the replies are read back out of the response
//! descriptor. Nothing here reaches into device internals, so the command
//! structs, the descriptor handling and the reply path are all covered.

use std::sync::{Arc, Mutex};

use ferrumvm::{
    devices::virtio::{
        devices::gpu::VirtioGpu,
        virtio::{
            VIRTQ_DESC_F_NEXT, VIRTQ_DESC_F_WRITE, VirtioDevice, VirtioGuestMemoryHandle,
            VirtioQueue,
        },
    },
    machine_config::memory_region::{GuestMemoryHandle, MemoryRegion},
    platform::display::{DisplayBackend, DisplayRect},
};

// --- guest memory layout -----------------------------------------------------

const MEM_SIZE: usize = 0x10000;

const DESC_OFF: u64 = 0x1000;
const AVAIL_OFF: u64 = 0x2000;
const USED_OFF: u64 = 0x3000;
const QUEUE_SIZE: u16 = 16;

/// Scratch area for request buffers.
const REQ_OFF: u64 = 0x4000;
/// Scratch area the device writes its replies into.
const RESP_OFF: u64 = 0x6000;
const RESP_MAX: usize = 512;
/// Staging area a guest would normally back a resource with.
const BACKING_OFF: u64 = 0x8000;
const BACKING_MAX: usize = 0x1000;

// --- wire-format constants ---------------------------------------------------

const CMD_GET_DISPLAY_INFO: u32 = 0x0100;
const CMD_RESOURCE_CREATE_2D: u32 = 0x0101;
const CMD_RESOURCE_UNREF: u32 = 0x0102;
const CMD_SET_SCANOUT: u32 = 0x0103;
const CMD_RESOURCE_FLUSH: u32 = 0x0104;
const CMD_TRANSFER_TO_HOST_2D: u32 = 0x0105;
const CMD_RESOURCE_ATTACH_BACKING: u32 = 0x0106;
const CMD_RESOURCE_DETACH_BACKING: u32 = 0x0107;
const CMD_GET_EDID: u32 = 0x010A;
const CMD_UPDATE_CURSOR: u32 = 0x010B;

const RESP_OK_NODATA: u32 = 0x1100;
const RESP_OK_DISPLAY_INFO: u32 = 0x1101;
const RESP_ERR_UNSUPPORTED: u32 = 0x1201;
const RESP_ERR_INVALID_ARGUMENT: u32 = 0x1202;
const RESP_ERR_INVALID_RESOURCE_ID: u32 = 0x1203;

const FORMAT_B8G8R8A8_UNORM: u32 = 1;

/// `virtio_gpu_ctrl_hdr` is 24 bytes: typ, flags, fence_id, ctx_id, ring_idx.
const HDR_LEN: usize = 24;

/// Descriptor 0 is the request, descriptor 1 the response.
const DESC_REQ: u16 = 0;
const DESC_RESP: u16 = 1;

// --- mock display backend ----------------------------------------------------

/// Records what the device tried to draw, so a test can assert on it without
/// needing a real window.
#[derive(Default)]
struct MockDisplay {
    width: u32,
    height: u32,
    blits: Vec<Blit>,
    presents: usize,
}

#[derive(Clone)]
struct Blit {
    rect: DisplayRect,
    stride: usize,
    pixels: Vec<u8>,
}

/// Forwards to a shared mock so the test can inspect it after the device has
/// taken ownership of the backend.
struct Proxy(Arc<Mutex<MockDisplay>>);

impl DisplayBackend for Proxy {
    fn resize_display(&mut self, width: u32, height: u32) -> bool {
        let mut mock = self.0.lock().unwrap();
        mock.width = width;
        mock.height = height;
        true
    }

    fn get_display_size(&self) -> (u32, u32) {
        let mock = self.0.lock().unwrap();
        (mock.width, mock.height)
    }

    fn upload(&mut self, _framebuffer: &[u8], _width: u32, _height: u32, _stride: u32) {}

    fn blit(&mut self, src: &[u8], src_stride: usize, src_rect: DisplayRect) {
        self.0.lock().unwrap().blit(src, src_stride, src_rect)
    }

    fn present(&mut self) {
        self.0.lock().unwrap().presents += 1;
    }
}

impl MockDisplay {
    fn blit(&mut self, src: &[u8], src_stride: usize, src_rect: DisplayRect) {
        // Mirrors what the real backend does: pull the rect out of the
        // stride-major source buffer.
        let row_len = src_rect.width as usize * 4;
        let mut pixels = Vec::with_capacity(row_len * src_rect.height as usize);
        for row in 0..src_rect.height as usize {
            let start = (src_rect.y as usize + row) * src_stride + src_rect.x as usize * 4;
            if start >= src.len() {
                break;
            }
            let len = row_len.min(src.len() - start);
            pixels.extend_from_slice(&src[start..start + len]);
        }

        self.blits.push(Blit {
            rect: src_rect,
            stride: src_stride,
            pixels,
        });
    }
}

// --- harness -----------------------------------------------------------------

struct GpuHarness {
    vmem: VirtioGuestMemoryHandle,
    queue: VirtioQueue,
    dev: VirtioGpu,
    display: Arc<Mutex<MockDisplay>>,
    /// How many requests we have submitted, i.e. what `avail.idx` should be.
    submitted: u16,
}

impl GpuHarness {
    fn new(width: u32, height: u32) -> Self {
        let boxed: Box<[u8]> = vec![0u8; MEM_SIZE].into_boxed_slice();
        let ptr = Box::into_raw(boxed) as *mut u8;
        let region = MemoryRegion::new(ptr, MEM_SIZE, 0);
        let mem: GuestMemoryHandle = Arc::new(Mutex::new(vec![region]));
        let vmem = VirtioGuestMemoryHandle::new(mem);

        let display = Arc::new(Mutex::new(MockDisplay {
            width,
            height,
            ..Default::default()
        }));

        let backend: Arc<Mutex<Box<dyn DisplayBackend + Send>>> =
            Arc::new(Mutex::new(Box::new(Proxy(Arc::clone(&display)))));
        let mut dev = VirtioGpu::new(Arc::clone(&backend));
        dev.pass_guest_memory(vmem.clone());

        let mut queue = VirtioQueue::new();
        queue.size = QUEUE_SIZE;
        queue.ready = true;
        queue.desc_addr = DESC_OFF;
        queue.avail_addr = AVAIL_OFF;
        queue.used_addr = USED_OFF;

        Self {
            vmem,
            queue,
            dev,
            display,
            submitted: 0,
        }
    }

    fn write_desc(&self, index: u16, addr: u64, len: u32, flags: u16, next: u16) {
        let base = DESC_OFF + (index as u64) * 16;
        let mut h = self.vmem.clone();
        h.write_u32(base, addr as u32);
        h.write_u32(base + 4, (addr >> 32) as u32);
        h.write_u32(base + 8, len);
        h.write_u16(base + 12, flags);
        h.write_u16(base + 14, next);
    }

    /// Publishes `head` on the avail ring and ticks the control queue.
    fn notify(&mut self, head: u16) -> bool {
        self.submitted += 1;
        let slot = (self.submitted - 1) % QUEUE_SIZE;
        self.vmem.write_u16(AVAIL_OFF + 2, self.submitted);
        self.vmem.write_u16(AVAIL_OFF + 4 + (slot as u64) * 2, head);

        self.dev.tick(0, &mut self.queue)
    }

    /// Submits one request/response descriptor pair and returns the `type` of
    /// the response the device wrote.
    fn submit(&mut self, request: &[u8]) -> u32 {
        self.vmem.write_guest_memory(REQ_OFF, request);
        self.vmem.write_guest_memory(RESP_OFF, &vec![0u8; RESP_MAX]);

        self.write_desc(
            DESC_REQ,
            REQ_OFF,
            request.len() as u32,
            VIRTQ_DESC_F_NEXT,
            DESC_RESP,
        );
        self.write_desc(DESC_RESP, RESP_OFF, RESP_MAX as u32, VIRTQ_DESC_F_WRITE, 0);

        self.notify(DESC_REQ);
        self.reply_type()
    }

    /// Submits a raw descriptor layout, for the malformed-chain tests.
    fn submit_raw(&mut self, req_len: u32, req_flags: u16, req_next: u16, write_req: &[u8]) -> u32 {
        if !write_req.is_empty() {
            self.vmem.write_guest_memory(REQ_OFF, write_req);
        }
        self.vmem.write_guest_memory(RESP_OFF, &vec![0u8; RESP_MAX]);

        self.write_desc(DESC_REQ, REQ_OFF, req_len, req_flags, req_next);
        self.write_desc(DESC_RESP, RESP_OFF, RESP_MAX as u32, VIRTQ_DESC_F_WRITE, 0);

        self.notify(DESC_REQ);
        self.reply_type()
    }

    /// The `type` field of the response header.
    fn reply_type(&self) -> u32 {
        self.vmem.read_u32(RESP_OFF)
    }

    fn reply_bytes(&self, len: usize) -> Vec<u8> {
        (0..len)
            .map(|i| self.vmem.read_byte(RESP_OFF + i as u64))
            .collect()
    }

    /// `avail.idx` as the device left it: the number of retired descriptors.
    fn used_idx(&self) -> u16 {
        self.vmem.read_u16(USED_OFF + 2)
    }

    fn blits(&self) -> Vec<Blit> {
        self.display.lock().unwrap().blits.clone()
    }

    fn presents(&self) -> usize {
        self.display.lock().unwrap().presents
    }
}

// --- request builders --------------------------------------------------------

fn hdr(typ: u32) -> Vec<u8> {
    let mut v = Vec::with_capacity(HDR_LEN);
    v.extend_from_slice(&typ.to_le_bytes()); // typ
    v.extend_from_slice(&0u32.to_le_bytes()); // flags
    v.extend_from_slice(&0u64.to_le_bytes()); // fence_id
    v.extend_from_slice(&0u32.to_le_bytes()); // ctx_id
    v.extend_from_slice(&0u32.to_le_bytes()); // ring_idx
    v
}

fn create_2d(id: u32, format: u32, width: u32, height: u32) -> Vec<u8> {
    let mut v = hdr(CMD_RESOURCE_CREATE_2D);
    v.extend_from_slice(&id.to_le_bytes());
    v.extend_from_slice(&format.to_le_bytes());
    v.extend_from_slice(&width.to_le_bytes());
    v.extend_from_slice(&height.to_le_bytes());
    v
}

fn unref(id: u32) -> Vec<u8> {
    let mut v = hdr(CMD_RESOURCE_UNREF);
    v.extend_from_slice(&id.to_le_bytes());
    v
}

fn set_scanout(rect: DisplayRect, scanout_id: u32, resource_id: u32) -> Vec<u8> {
    let mut v = hdr(CMD_SET_SCANOUT);
    v.extend_from_slice(&rect.x.to_le_bytes());
    v.extend_from_slice(&rect.y.to_le_bytes());
    v.extend_from_slice(&rect.width.to_le_bytes());
    v.extend_from_slice(&rect.height.to_le_bytes());
    v.extend_from_slice(&scanout_id.to_le_bytes());
    v.extend_from_slice(&resource_id.to_le_bytes());
    v
}

fn flush(rect: DisplayRect, resource_id: u32) -> Vec<u8> {
    let mut v = hdr(CMD_RESOURCE_FLUSH);
    v.extend_from_slice(&rect.x.to_le_bytes());
    v.extend_from_slice(&rect.y.to_le_bytes());
    v.extend_from_slice(&rect.width.to_le_bytes());
    v.extend_from_slice(&rect.height.to_le_bytes());
    v.extend_from_slice(&resource_id.to_le_bytes());
    v
}

fn transfer_to_host(rect: DisplayRect, offset: u64, resource_id: u32) -> Vec<u8> {
    let mut v = hdr(CMD_TRANSFER_TO_HOST_2D);
    v.extend_from_slice(&rect.x.to_le_bytes());
    v.extend_from_slice(&rect.y.to_le_bytes());
    v.extend_from_slice(&rect.width.to_le_bytes());
    v.extend_from_slice(&rect.height.to_le_bytes());
    v.extend_from_slice(&offset.to_le_bytes());
    v.extend_from_slice(&resource_id.to_le_bytes());
    v
}

fn attach_backing(id: u32, entries: &[(u64, u32)]) -> Vec<u8> {
    let mut v = hdr(CMD_RESOURCE_ATTACH_BACKING);
    v.extend_from_slice(&id.to_le_bytes());
    v.extend_from_slice(&(entries.len() as u32).to_le_bytes());
    for (addr, len) in entries {
        v.extend_from_slice(&addr.to_le_bytes());
        v.extend_from_slice(&len.to_le_bytes());
        v.extend_from_slice(&0u32.to_le_bytes()); // padding
    }
    v
}

fn detach_backing(id: u32) -> Vec<u8> {
    let mut v = hdr(CMD_RESOURCE_DETACH_BACKING);
    v.extend_from_slice(&id.to_le_bytes());
    v
}

/// Creates a resource, attaches `size` bytes of backing, and stages a pattern
/// in it: one repeated byte per row.
fn stage_resource(h: &mut GpuHarness, id: u32, width: u32, height: u32) -> Vec<u8> {
    let size = (width * height * 4) as usize;
    assert!(size <= BACKING_MAX);

    assert_eq!(
        h.submit(&create_2d(id, FORMAT_B8G8R8A8_UNORM, width, height)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&attach_backing(id, &[(BACKING_OFF, size as u32)])),
        RESP_OK_NODATA
    );

    let pixels: Vec<u8> = (0..size)
        .map(|i| (i / (width as usize * 4)) as u8)
        .collect();
    h.vmem.write_guest_memory(BACKING_OFF, &pixels);
    pixels
}

// --- tests -------------------------------------------------------------------

#[test]
fn get_display_info_reports_the_backend_size() {
    let mut h = GpuHarness::new(1280, 720);

    assert_eq!(h.submit(&hdr(CMD_GET_DISPLAY_INFO)), RESP_OK_DISPLAY_INFO);

    // hdr (24 bytes) then virtio_gpu_display_info: rect (16) enabled (4) flags (4)
    let body = h.reply_bytes(HDR_LEN + 24);
    assert_eq!(u32::from_le_bytes(body[24..28].try_into().unwrap()), 0); // x
    assert_eq!(u32::from_le_bytes(body[28..32].try_into().unwrap()), 0); // y
    assert_eq!(u32::from_le_bytes(body[32..36].try_into().unwrap()), 1280);
    assert_eq!(u32::from_le_bytes(body[36..40].try_into().unwrap()), 720);
    assert_eq!(u32::from_le_bytes(body[40..44].try_into().unwrap()), 1); // enabled

    assert_eq!(h.used_idx(), 1);
}

#[test]
fn get_edid_is_answered_instead_of_stalling() {
    let mut h = GpuHarness::new(64, 64);
    // The driver blocks for its full response timeout if this never replies.
    // This was a 5.17s stall between "number of cap sets: 0" and
    // "Initialized virtio_gpu".
    assert_eq!(h.submit(&hdr(CMD_GET_EDID)), RESP_OK_NODATA);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn tick_reports_that_it_did_work() {
    let mut h = GpuHarness::new(64, 64);

    h.write_desc(
        DESC_REQ,
        REQ_OFF,
        HDR_LEN as u32,
        VIRTQ_DESC_F_NEXT,
        DESC_RESP,
    );
    h.write_desc(DESC_RESP, RESP_OFF, RESP_MAX as u32, VIRTQ_DESC_F_WRITE, 0);
    h.vmem.write_guest_memory(REQ_OFF, &hdr(CMD_GET_EDID));

    // If this is false the transport never raises interrupt_status and the
    // guest only ever learns of completions by polling.
    assert!(h.notify(DESC_REQ), "tick must report the completion");

    // Nothing new published, so nothing to do.
    assert!(!h.dev.tick(0, &mut h.queue));
}

#[test]
fn unknown_command_gets_an_error_response() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(h.submit(&hdr(0xDEAD_BEEF)), RESP_ERR_UNSUPPORTED);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn create_2d_succeeds_and_then_rejects_a_duplicate_id() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 16, 16)),
        RESP_OK_NODATA
    );
    // Re-using a live id used to silently orphan the old resource's pages.
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 16, 16)),
        RESP_ERR_INVALID_RESOURCE_ID
    );
}

#[test]
fn create_2d_rejects_absurd_geometry_without_aborting() {
    let mut h = GpuHarness::new(64, 64);
    // 0xFFFF x 0xFFFF at 4bpp is ~16GiB; an unbounded `vec![0; ..]` aborts
    // the whole VMM when the allocation fails.
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 0xFFFF, 0xFFFF)),
        RESP_ERR_INVALID_ARGUMENT
    );
    assert_eq!(
        h.submit(&create_2d(2, FORMAT_B8G8R8A8_UNORM, 1 << 20, 1 << 20)),
        RESP_ERR_INVALID_ARGUMENT
    );
    assert_eq!(
        h.submit(&create_2d(3, FORMAT_B8G8R8A8_UNORM, 0, 16)),
        RESP_ERR_INVALID_ARGUMENT
    );
    assert_eq!(
        h.submit(&create_2d(4, FORMAT_B8G8R8A8_UNORM, 16, 0)),
        RESP_ERR_INVALID_ARGUMENT
    );
}

#[test]
fn create_2d_rejects_unsupported_formats() {
    let mut h = GpuHarness::new(64, 64);
    // Storing a non-32bpp format with a 4bpp stride would garble the output.
    assert_eq!(
        h.submit(&create_2d(1, 0, 16, 16)),
        RESP_ERR_INVALID_ARGUMENT
    );
    assert_eq!(
        h.submit(&create_2d(1, 7, 16, 16)),
        RESP_ERR_INVALID_ARGUMENT
    );
}

#[test]
fn resources_are_capped() {
    let mut h = GpuHarness::new(64, 64);
    // 1x1 resources so the count cap, not the byte budget, is what bites.
    for id in 1..=80 {
        h.submit(&create_2d(id, FORMAT_B8G8R8A8_UNORM, 1, 1));
    }

    // The first 64 exist...
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 1, 1), 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 1, 1), 64)),
        RESP_OK_NODATA
    );
    // ...and new ones are refused rather than growing without bound.
    assert_eq!(
        h.submit(&create_2d(100, FORMAT_B8G8R8A8_UNORM, 1, 1)),
        RESP_ERR_INVALID_ARGUMENT
    );
}

#[test]
fn unref_releases_the_resource_and_detaches_it_from_scanouts() {
    let mut h = GpuHarness::new(64, 64);
    let pixels = stage_resource(&mut h, 1, 4, 4);
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );

    assert_eq!(h.submit(&unref(1)), RESP_OK_NODATA);

    // The resource is gone, and so is the scanout that pointed at it.
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_ERR_INVALID_RESOURCE_ID
    );
    assert!(h.blits().is_empty());
    assert_eq!(pixels.len(), 64);
}

#[test]
fn set_scanout_validates_its_arguments() {
    let mut h = GpuHarness::new(64, 64);
    // scanout_id past the end of the table.
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 16, 16), 999, 0)),
        RESP_ERR_INVALID_ARGUMENT
    );
    // resource that does not exist.
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 16, 16), 0, 42)),
        RESP_ERR_INVALID_RESOURCE_ID
    );
    // resource_id 0 is the documented "disable this scanout" value.
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 16, 16), 0, 0)),
        RESP_OK_NODATA
    );
}

#[test]
fn flush_reaches_a_scanout_past_the_unbound_ones() {
    let mut h = GpuHarness::new(64, 64);
    let _ = stage_resource(&mut h, 1, 4, 4);
    // Point scanout 3 at resource 1, leaving 0..2 unbound.
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 3, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_OK_NODATA
    );

    // Bailing out on the first unbound scanout would draw nothing here.
    let blits = h.blits();
    assert_eq!(blits.len(), 1);
    assert_eq!(blits[0].rect, DisplayRect::new(0, 0, 4, 4));
    assert_eq!(blits[0].stride, 16);
    // The device's job ends at `blit`; presenting is the backend's business.
    assert_eq!(h.presents(), 0);
}

#[test]
fn flush_of_a_resource_no_scanout_uses_draws_nothing() {
    let mut h = GpuHarness::new(64, 64);
    let _ = stage_resource(&mut h, 1, 4, 4);
    // No set_scanout at all.
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_OK_NODATA
    );
    assert!(h.blits().is_empty());
}

#[test]
fn transfer_to_host_moves_guest_pixels_into_the_resource() {
    let mut h = GpuHarness::new(64, 64);
    let pixels = stage_resource(&mut h, 1, 4, 4);

    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_OK_NODATA
    );

    // End to end: bytes staged in guest RAM come out the other side intact.
    let blits = h.blits();
    assert_eq!(blits.len(), 1);
    assert_eq!(blits[0].pixels, pixels);
}

#[test]
fn transfer_to_host_honours_a_sub_rectangle() {
    let mut h = GpuHarness::new(64, 64);
    let _ = stage_resource(&mut h, 1, 4, 4);

    // Pull rows 2..4 out of a 4x4 resource.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 2, 4, 2), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 2, 4, 2), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 2, 4, 2), 1)),
        RESP_OK_NODATA
    );

    let blits = h.blits();
    assert_eq!(blits.len(), 1);
    // 4x2 at 4bpp is 32 bytes. stage_resource writes one repeated byte per
    // row, and this rect covers rows 2 and 3.
    assert_eq!(blits[0].rect, DisplayRect::new(0, 2, 4, 2));
    assert_eq!(blits[0].pixels, [vec![2u8; 16], vec![3u8; 16]].concat());
}

#[test]
fn transfer_to_host_rejects_a_rect_outside_the_resource() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&attach_backing(1, &[(BACKING_OFF, 64)])),
        RESP_OK_NODATA
    );

    // Reads past the end of the shadow buffer.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 64, 64), 0, 1)),
        RESP_ERR_INVALID_ARGUMENT
    );
    // Negative-looking origin.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(8, 0, 4, 4), 0, 1)),
        RESP_ERR_INVALID_ARGUMENT
    );
    // Empty rect.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 0, 4), 0, 1)),
        RESP_ERR_INVALID_ARGUMENT
    );
    // Offset that overflows the address arithmetic.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), u64::MAX, 1)),
        RESP_ERR_INVALID_ARGUMENT
    );
    // Unknown resource.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 9)),
        RESP_ERR_INVALID_RESOURCE_ID
    );
}

#[test]
fn transfer_to_host_survives_backing_far_smaller_than_the_resource() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 64, 64)),
        RESP_OK_NODATA
    );
    // One 4KiB page backing a 16KiB resource: the read has to stop, not run off.
    assert_eq!(
        h.submit(&attach_backing(1, &[(BACKING_OFF, 4096)])),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 64, 64), 0, 1)),
        RESP_OK_NODATA
    );
}

#[test]
fn transfer_to_host_works_with_no_backing_at_all() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );
    // No attach_backing: the resource is just zero filled.
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
}

#[test]
fn transfer_to_host_walks_multiple_backing_pages() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );
    // Two 32-byte pages, each filled with a distinct byte.
    h.vmem.write_guest_memory(BACKING_OFF, &[0x11u8; 32]);
    h.vmem.write_guest_memory(BACKING_OFF + 32, &[0x22u8; 32]);
    assert_eq!(
        h.submit(&attach_backing(
            1,
            &[(BACKING_OFF, 32), (BACKING_OFF + 32, 32)]
        )),
        RESP_OK_NODATA
    );

    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_OK_NODATA
    );

    // 4x4 at 4bpp is 64 bytes. Rows 0-1 come from the first page, rows 2-3
    // from the second, so the walk across the page boundary has to stitch them
    // together in order.
    let blits = h.blits();
    assert_eq!(blits.len(), 1);
    assert_eq!(
        blits[0].pixels,
        [vec![0x11u8; 32], vec![0x22u8; 32]].concat()
    );
}

#[test]
fn detach_backing_stops_the_resource_reading_guest_memory() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&attach_backing(1, &[(BACKING_OFF, 64)])),
        RESP_OK_NODATA
    );
    assert_eq!(h.submit(&detach_backing(1)), RESP_OK_NODATA);

    h.vmem.write_guest_memory(BACKING_OFF, &[0xAB; 64]);
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_OK_NODATA
    );

    // Nothing left to read, so the resource is still blank.
    let blits = h.blits();
    assert_eq!(blits.len(), 1);
    assert_eq!(blits[0].pixels, vec![0u8; 64]);
}

#[test]
fn backing_commands_on_an_unknown_resource_are_rejected() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&attach_backing(7, &[(BACKING_OFF, 64)])),
        RESP_ERR_INVALID_RESOURCE_ID
    );
    assert_eq!(h.submit(&detach_backing(7)), RESP_ERR_INVALID_RESOURCE_ID);
}

#[test]
fn attach_backing_ignores_a_short_entry_list() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );

    // Claim four entries but supply one: take what is there, do not read past
    // the end of the request.
    let mut req = hdr(CMD_RESOURCE_ATTACH_BACKING);
    req.extend_from_slice(&1u32.to_le_bytes()); // resource_id
    req.extend_from_slice(&4u32.to_le_bytes()); // nr_entries
    req.extend_from_slice(&BACKING_OFF.to_le_bytes());
    req.extend_from_slice(&64u32.to_le_bytes());
    req.extend_from_slice(&0u32.to_le_bytes()); // padding
    assert_eq!(h.submit(&req), RESP_OK_NODATA);

    h.vmem.write_guest_memory(BACKING_OFF, &[0x33; 64]);
    assert_eq!(
        h.submit(&transfer_to_host(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );
}

#[test]
fn a_head_descriptor_without_next_is_still_retired() {
    let mut h = GpuHarness::new(64, 64);

    // No NEXT flag: unusable, but it must still land in the used ring or the
    // avail and used indices drift apart and the driver hangs.
    let reply = h.submit_raw(HDR_LEN as u32, 0, 0, &hdr(CMD_GET_EDID));

    assert_eq!(reply, 0, "no response should have been written");
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn a_writable_head_descriptor_is_rejected() {
    let mut h = GpuHarness::new(64, 64);

    let reply = h.submit_raw(
        HDR_LEN as u32,
        VIRTQ_DESC_F_NEXT | VIRTQ_DESC_F_WRITE,
        DESC_RESP,
        &hdr(CMD_GET_EDID),
    );

    assert_eq!(reply, 0);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn a_response_descriptor_past_the_end_of_the_table_is_rejected() {
    let mut h = GpuHarness::new(64, 64);

    // next = 6000 with a 16-entry table: the device would otherwise read a
    // descriptor out of bounds.
    let reply = h.submit_raw(HDR_LEN as u32, VIRTQ_DESC_F_NEXT, 6000, &hdr(CMD_GET_EDID));

    assert_eq!(reply, 0);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn an_oversized_request_descriptor_is_rejected() {
    let mut h = GpuHarness::new(64, 64);

    // A guest-controlled length must not become a multi-gigabyte read.
    let reply = h.submit_raw(0xFFFF_FFFF, VIRTQ_DESC_F_NEXT, DESC_RESP, &[]);

    assert_eq!(reply, 0);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn a_truncated_request_is_rejected() {
    let mut h = GpuHarness::new(64, 64);

    // Says RESOURCE_CREATE_2D but is not long enough to hold the struct.
    let reply = h.submit_raw(
        8,
        VIRTQ_DESC_F_NEXT,
        DESC_RESP,
        &hdr(CMD_RESOURCE_CREATE_2D),
    );

    assert_eq!(reply, 0);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn the_cursor_queue_retires_descriptors_and_replies() {
    let mut h = GpuHarness::new(64, 64);

    h.write_desc(
        DESC_REQ,
        REQ_OFF,
        HDR_LEN as u32,
        VIRTQ_DESC_F_NEXT,
        DESC_RESP,
    );
    h.write_desc(DESC_RESP, RESP_OFF, RESP_MAX as u32, VIRTQ_DESC_F_WRITE, 0);
    h.vmem.write_guest_memory(REQ_OFF, &hdr(CMD_UPDATE_CURSOR));
    h.vmem.write_guest_memory(RESP_OFF, &vec![0u8; RESP_MAX]);

    h.submitted += 1;
    h.vmem.write_u16(AVAIL_OFF + 2, h.submitted);
    h.vmem.write_u16(AVAIL_OFF + 4, DESC_REQ);

    // Dropping the descriptors without replying wedges the cursor queue.
    assert!(h.dev.tick(1, &mut h.queue));
    assert_eq!(h.used_idx(), 1);
    assert_eq!(h.reply_type(), RESP_OK_NODATA);
}

#[test]
fn the_cursor_queue_rejects_commands_it_does_not_know() {
    let mut h = GpuHarness::new(64, 64);

    h.write_desc(
        DESC_REQ,
        REQ_OFF,
        HDR_LEN as u32,
        VIRTQ_DESC_F_NEXT,
        DESC_RESP,
    );
    h.write_desc(DESC_RESP, RESP_OFF, RESP_MAX as u32, VIRTQ_DESC_F_WRITE, 0);
    h.vmem.write_guest_memory(REQ_OFF, &hdr(CMD_RESOURCE_FLUSH));

    h.submitted += 1;
    h.vmem.write_u16(AVAIL_OFF + 2, h.submitted);
    h.vmem.write_u16(AVAIL_OFF + 4, DESC_REQ);

    h.dev.tick(1, &mut h.queue);
    assert_eq!(h.reply_type(), RESP_ERR_UNSUPPORTED);
    assert_eq!(h.used_idx(), 1);
}

#[test]
fn tick_on_an_unknown_queue_is_a_no_op() {
    let mut h = GpuHarness::new(64, 64);
    assert!(!h.dev.tick(5, &mut h.queue));
}

#[test]
fn tick_without_guest_memory_is_a_no_op() {
    let display = Arc::new(Mutex::new(MockDisplay::default()));
    let backend: Arc<Mutex<Box<dyn DisplayBackend + Send>>> =
        Arc::new(Mutex::new(Box::new(Proxy(display))));
    let mut dev = VirtioGpu::new(Arc::clone(&backend));
    let mut queue = VirtioQueue::new();

    // `pass_guest_memory` was never called.
    assert!(!dev.tick(0, &mut queue));
}

#[test]
fn reset_drops_resources_and_scanouts() {
    let mut h = GpuHarness::new(64, 64);
    assert_eq!(
        h.submit(&create_2d(1, FORMAT_B8G8R8A8_UNORM, 4, 4)),
        RESP_OK_NODATA
    );
    assert_eq!(
        h.submit(&set_scanout(DisplayRect::new(0, 0, 4, 4), 0, 1)),
        RESP_OK_NODATA
    );

    h.dev.reset();

    // After a guest reset the old ids and the guest pages behind them are gone.
    assert_eq!(
        h.submit(&flush(DisplayRect::new(0, 0, 4, 4), 1)),
        RESP_ERR_INVALID_RESOURCE_ID
    );
    assert!(h.blits().is_empty());
}

#[test]
fn config_is_readable_and_writable_within_bounds() {
    let mut h = GpuHarness::new(64, 64);

    // events_read, events_clear, num_scanouts, num_capsets
    let cfg = h.dev.read_config(16);
    assert_eq!(u32::from_le_bytes(cfg[0..4].try_into().unwrap()), 0);
    assert_eq!(u32::from_le_bytes(cfg[8..12].try_into().unwrap()), 1); // 1 scanout
    assert_eq!(u32::from_le_bytes(cfg[12..16].try_into().unwrap()), 0); // no capsets

    // A partial write into events_clear.
    h.dev.write_config(4, &[0xAA, 0xBB]);
    let cfg = h.dev.read_config(16);
    assert_eq!(
        u32::from_le_bytes(cfg[4..8].try_into().unwrap()),
        0x0000_BBAA
    );

    // Read-only fields are ignored.
    h.dev.write_config(8, &1u32.to_le_bytes());
    assert_eq!(
        u32::from_le_bytes(h.dev.read_config(16)[8..12].try_into().unwrap()),
        1
    );
}

#[test]
fn config_writes_wider_than_a_word_do_not_panic() {
    let mut h = GpuHarness::new(64, 64);

    // An 8 byte MMIO write at config offset 0 used to index past the end of a
    // [u8; 4] scratch buffer and abort the VMM.
    h.dev.write_config(0, &[1, 2, 3, 4, 5, 6, 7, 8]);
    h.dev.write_config(2, &[1, 2, 3, 4, 5, 6, 7, 8]);
    h.dev.write_config(0, &[]);

    // Whatever happened, the device is still usable.
    assert_eq!(h.submit(&hdr(CMD_GET_EDID)), RESP_OK_NODATA);
}

#[test]
fn config_short_reads_are_padded() {
    let h = GpuHarness::new(64, 64);
    assert_eq!(h.dev.read_config(4).len(), 4);
    assert_eq!(h.dev.read_config(2).len(), 2);
}
