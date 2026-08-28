use ferrumvm::machine_config::memory_region::MemoryRegion;

fn alloc_region(size: usize) -> MemoryRegion {
    let layout = std::alloc::Layout::from_size_align(size, 1).unwrap();
    let ptr = unsafe { std::alloc::alloc(layout) };
    assert!(!ptr.is_null(), "safe alloc failed");
    MemoryRegion::new(ptr, size, 0)
}

fn drop_region(region: &MemoryRegion) {
    if !region.ptr.is_null() {
        unsafe {
            let layout = std::alloc::Layout::from_size_align(region.mem_size, 1).unwrap();
            std::alloc::dealloc(region.ptr, layout);
        }
    }
}

#[test]
fn write_then_read_roundtrip() {
    let region = alloc_region(64);
    let data = vec![1, 2, 3, 4, 5];
    region.write(&data, 10);
    let out = region.read(10, 5).unwrap();
    assert_eq!(out, data);
    drop_region(&region);
}

#[test]
fn write_out_of_bounds_is_ignored() {
    let region = alloc_region(16);
    region.write(&[9, 9, 9], 15); // 15 + 3 > 16
    let out = region.read(14, 2).unwrap();
    // untouched - remains zero-initialized
    assert_eq!(out, vec![0, 0]);
    drop_region(&region);
}

#[test]
fn read_out_of_bounds_returns_none() {
    let region = alloc_region(16);
    assert!(region.read(10, 10).is_none()); // 10 + 10 > 16
    assert!(region.read(0, 17).is_none());
    drop_region(&region);
}

#[test]
fn zero_length_read_at_valid_addr_returns_empty() {
    let region = alloc_region(16);
    // addr within bounds: 0 + 0 <= 16
    assert_eq!(region.read(0, 0), Some(vec![]));
    drop_region(&region);
}

#[test]
fn write_to_null_pointer_is_noop() {
    let region = MemoryRegion::new(std::ptr::null_mut(), 16, 0);
    region.write(&[1], 0); // must not panic
    assert!(region.read(0, 1).is_none());
}

#[test]
fn mem_offset_is_preserved() {
    let region = MemoryRegion::new(1 as *mut u8, 4096, 0xF0000000);
    assert_eq!(region.mem_offset, 0xF0000000);
    assert_eq!(region.mem_size, 4096);
}

// Boxed memory (sound and simple for testing): alloc via Box<[u8]>.
fn boxed_region(size: usize, offset: u64) -> MemoryRegion {
    let boxed: Box<[u8]> = vec![0u8; size].into_boxed_slice();
    let ptr = Box::into_raw(boxed) as *mut u8;
    MemoryRegion::new(ptr, size, offset)
}

#[test]
fn read_respects_offset_and_returns_contiguous_bytes() {
    let region = boxed_region(8, 0x1000);
    let input = [0xDE, 0xAD, 0xBE, 0xEF, 0x01, 0x02, 0x03, 0x04];
    region.write(&input, 0);
    // multi-region style reads are handled by the virtio handle; here just a slice
    let out = region.read(0, 8).unwrap();
    assert_eq!(out.as_slice(), &input);
}
