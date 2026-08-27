use std::sync::{Arc, Mutex};

use ferrumvm::{
    devices::virtio::{
        devices::{counter::CntVirtio, rng::RngVirtio, console::ConsoleVirtio},
        virtio::{VirtioDevice, VirtioGuestMemoryHandle, VirtioQueue},
    },
    memory_region::{GuestMemoryHandle, MemoryRegion},
};

const MEM_SIZE: usize = 0x10000;

// Layout of our synthetic guest memory used to host a virtio queue.
const DESC_OFF: u64 = 0x1000;
const AVAIL_OFF: u64 = 0x2000;
const USED_OFF: u64 = 0x3000;
const QUEUE_SIZE: u16 = 16;

struct TestHarness {
    mem: GuestMemoryHandle,
    vmem: VirtioGuestMemoryHandle,
}

impl TestHarness {
    fn new() -> Self {
        let boxed: Box<[u8]> = vec![0u8; MEM_SIZE].into_boxed_slice();
        let ptr = Box::into_raw(boxed) as *mut u8;
        let region = MemoryRegion::new(ptr, MEM_SIZE, 0);
        let mem: GuestMemoryHandle = Arc::new(Mutex::new(vec![region]));
        let vmem = VirtioGuestMemoryHandle::new(mem.clone());
        Self { mem, vmem }
    }
}

fn new_queue() -> VirtioQueue {
    let mut q = VirtioQueue::new();
    q.size = QUEUE_SIZE;
    q.ready = true;
    q.desc_addr = DESC_OFF;
    q.avail_addr = AVAIL_OFF;
    q.used_addr = USED_OFF;
    q
}

fn write_desc(vmem: &VirtioGuestMemoryHandle, index: u16, addr: u64, len: u32, flags: u16, next: u16) {
    let base = DESC_OFF + (index as u64) * 16;
    let mut h = vmem.clone();
    h.write_u32(base, addr as u32);
    h.write_u32(base + 4, (addr >> 32) as u32);
    h.write_u32(base + 8, len);
    h.write_u16(base + 12, flags);
    h.write_u16(base + 14, next);
}

fn write_avail(harness: &mut TestHarness, idx: u16, heads: &[u16]) {
    // avail.flags @ AVAIL, avail.idx @ AVAIL+2, ring @ AVAIL+4
    harness.vmem.write_u16(AVAIL_OFF + 2, idx);
    for (i, head) in heads.iter().enumerate() {
        harness.vmem.write_u16(AVAIL_OFF + 4 + (i as u64) * 2, *head);
    }
}

fn read_key(region: &MemoryRegion, addr: usize) -> u8 {
    region.read(addr, 1).unwrap()[0]
}

#[test]
fn pop_avail_with_no_new_entries_returns_none() {
    let mut h = TestHarness::new();
    let mut q = new_queue();
    write_avail(&mut h, 0, &[]);
    assert!(q.pop_avail(&h.vmem).is_none());
}

#[test]
fn pop_avail_returns_heads_in_ring_order() {
    let mut h = TestHarness::new();
    let mut q = new_queue();
    write_avail(&mut h, 2, &[10, 11]);
    assert_eq!(q.pop_avail(&h.vmem), Some(10));
    assert_eq!(q.pop_avail(&h.vmem), Some(11));
    assert!(q.pop_avail(&h.vmem).is_none());
}

#[test]
fn pop_avail_wraps_around_ring() {
    let mut h = TestHarness::new();
    let mut q = new_queue();
    q.last_avail_idx = QUEUE_SIZE - 1;
    // head at ring slot (QUEUE_SIZE-1)
    h.vmem.write_u16(AVAIL_OFF + 4 + ((QUEUE_SIZE - 1) as u64) * 2, 0x77);
    write_avail(&mut h, QUEUE_SIZE, &[0x77]);
    assert_eq!(q.pop_avail(&h.vmem), Some(0x77));
}

#[test]
fn get_descriptor_reads_all_fields() {
    let h = TestHarness::new();
    write_desc(&h.vmem, 3, 0x4000, 512, 0x01, 4);
    let q = new_queue();
    let desc = q.get_descriptor(&h.vmem, 3);
    assert_eq!(desc.addr, 0x4000);
    assert_eq!(desc.len, 512);
    assert_eq!(desc.flags, 0x01);
    assert_eq!(desc.next, 4);
}

#[test]
fn push_used_writes_used_ring_and_increments_idx() {
    let mut h = TestHarness::new();
    let mut q = new_queue();
    q.push_used(&mut h.vmem, 5, 64);
    q.push_used(&mut h.vmem, 6, 128);

    // used ring: used.flags @ USED, used.idx @ USED+2, entries @ USED+4 (id) and +8 (len)
    assert_eq!(h.vmem.read_u16(USED_OFF + 2), 2);
    let first_id = h.vmem.read_u32(USED_OFF + 4);
    let first_len = h.vmem.read_u32(USED_OFF + 8);
    assert_eq!(first_id, 5);
    assert_eq!(first_len, 64);
    assert_eq!(q.last_used_idx, 2);
}

#[test]
fn counter_device_increments_input() {
    let mut h = TestHarness::new();
    let mut dev = CntVirtio::new();
    dev.pass_guest_memory(h.vmem.clone());

    let mut q = new_queue();
    // desc head -> input value 41 (device-readable), desc.next -> output buffer (device-writable)
    write_desc(&h.vmem, 0, 0x5000, 4, 0x03, 1); // head: read/write, next=1
    write_desc(&h.vmem, 1, 0x5010, 4, 0x02, 0); // write-only output
    h.vmem.write_u32(0x5000, 41);
    write_avail(&mut h, 1, &[0]);

    assert!(dev.tick(0, &mut q));
    assert_eq!(h.vmem.read_u32(0x5010), 42);
    // queue advanced
    assert_eq!(q.last_used_idx, 1);
}

#[test]
fn counter_device_ignores_other_queue_sel() {
    let h = TestHarness::new();
    let mut dev = CntVirtio::new();
    dev.pass_guest_memory(h.vmem.clone());
    let mut q = new_queue();
    assert!(!dev.tick(1, &mut q));
}

#[test]
fn rng_device_fills_write_only_buffer() {
    let mut h = TestHarness::new();
    let mut dev = RngVirtio::new();
    dev.pass_guest_memory(h.vmem.clone());

    let mut q = new_queue();
    write_desc(&h.vmem, 0, 0x6000, 16, 0x02, 0); // write-only
    write_avail(&mut h, 1, &[0]);

    assert!(dev.tick(0, &mut q));
    let data: Vec<u8> = (0..16u64).map(|i| h.vmem.read_byte(0x6000 + i)).collect();
    assert_eq!(data.len(), 16);
    // random bytes should not be trivially all-zero
    assert!(data.iter().any(|b| *b != 0));
    assert_eq!(q.last_used_idx, 1);
}

#[test]
fn console_device_selects_queue_by_sel() {
    let h = TestHarness::new();
    let mut dev = ConsoleVirtio::new();
    dev.pass_guest_memory(h.vmem.clone());
    let mut q = new_queue();
    assert!(dev.tick(0, &mut q)); // rx
    assert!(dev.tick(1, &mut q)); // tx
    assert!(!dev.tick(2, &mut q)); // invalid
}

#[test]
fn read_guest_memory_returns_bytes_and_fills_buffer() {
    let mut h = TestHarness::new();
    h.vmem.write_u8(0x7000, 0xDE);
    h.vmem.write_u8(0x7001, 0xAD);
    let mut buf = vec![0u8; 2];
    h.vmem.read_guest_memory(0x7000, &mut buf);
    assert_eq!(buf, vec![0xDE, 0xAD]);
}

#[test]
fn write_guest_memory_writes_bytes() {
    let mut h = TestHarness::new();
    h.vmem.write_guest_memory(0x7000, &[1, 2, 3]);
    assert_eq!(read_key(&h.mem.lock().unwrap()[0], 0x7000), 1);
    assert_eq!(h.vmem.read_byte(0x7002), 3);
}

#[test]
fn multi_width_endian_reads() {
    let mut h = TestHarness::new();
    // Write a 64-bit little-endian value split across four u16s and verify reads.
    h.vmem.write_u32(0x8000, 0x1122_3344);
    assert_eq!(h.vmem.read_u16(0x8000), 0x3344);
    assert_eq!(h.vmem.read_u32(0x8000), 0x1122_3344);
    assert_eq!(h.vmem.read_u64(0x8000), 0x1122_3344);
}
