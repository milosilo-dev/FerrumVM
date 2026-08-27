const MGK_NUM: u32 = 0xFE02FE02;
const MEM_TYPE_RESERVED: u32 = 0;
const MEM_TYPE_CONVENTIONAL: u32 = 7;
const MEM_TYPE_MMIO: u32 = 11;

#[test]
fn binary_new_stores_data_and_offset() {
    use ferrumvm::machine_config::binary::Binary;
    let bin = Binary::new(vec![1, 2, 3], 0x7000);
    assert_eq!(bin.data, vec![1, 2, 3]);
    assert_eq!(bin.offset, 0x7000);
}

#[test]
fn binary_reset_vector_has_expected_bytecode_and_offset() {
    use ferrumvm::machine_config::binary::Binary;
    let rv = Binary::reset_vector();
    // far jump to 0x7E00:0x0000
    assert_eq!(rv.data, vec![0xEA, 0x00, 0x7E, 0x00, 0x00]);
    assert_eq!(rv.offset, 0xFFF0);
}

#[test]
fn mem_type_variants_have_expected_values() {
    use ferrumvm::machine_config::mem_map::MemType;
    assert_eq!(MemType::Reserved as u32, MEM_TYPE_RESERVED);
    assert_eq!(MemType::ConventionalMemory as u32, MEM_TYPE_CONVENTIONAL);
    assert_eq!(MemType::MMIO as u32, MEM_TYPE_MMIO);
}

#[test]
fn memmap_as_bytes_is_little_endian_20_bytes() {
    use ferrumvm::machine_config::mem_map::{MemMap, MemType};
    let map = MemMap {
        start: 0x1000,
        end: 0x2000,
        mem_type: MemType::ConventionalMemory as u32,
    };
    let bytes = map.as_bytes();
    assert_eq!(bytes.len(), 20);
    // start, LE
    assert_eq!(&bytes[0..8], &0x1000u64.to_le_bytes());
    // end, LE
    assert_eq!(&bytes[8..16], &0x2000u64.to_le_bytes());
    // type, LE
    assert_eq!(&bytes[16..20], &MEM_TYPE_CONVENTIONAL.to_le_bytes());
}

#[test]
fn memmap_header_as_bytes_is_little_endian_8_bytes() {
    use ferrumvm::machine_config::mem_map::MemMapHeader;
    let header = MemMapHeader {
        mgk_num: MGK_NUM,
        length: 13,
    };
    let bytes = header.as_bytes();
    assert_eq!(bytes.len(), 8);
    assert_eq!(&bytes[0..4], &MGK_NUM.to_le_bytes());
    assert_eq!(&bytes[4..8], &13u32.to_le_bytes());
}
