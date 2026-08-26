use std::mem::size_of;

pub fn read_le_u32(input: &mut &[u8]) -> u32 {
    if input.len() < size_of::<u32>() {
        *input = &[];
        return 0;
    }

    let (int_bytes, rest) = input.split_at(size_of::<u32>());
    *input = rest;

    u32::from_le_bytes(int_bytes.try_into().unwrap())
}

pub fn read_le_u64(input: &mut &[u8]) -> u64 {
    if input.len() < size_of::<u64>() {
        *input = &[];
        return 0;
    }

    let (int_bytes, rest) = input.split_at(size_of::<u64>());
    *input = rest;

    u64::from_le_bytes(int_bytes.try_into().unwrap())
}
