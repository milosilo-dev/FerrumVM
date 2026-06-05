use crate::machine_config::binary::Binary;

pub fn build_xsdt(table_addrs: &[u64]) -> Binary {
    let total_len = 36 + table_addrs.len() * 8;
    let mut xsdt = vec![0u8; total_len];

    xsdt[0..4].copy_from_slice(b"XSDT");
    xsdt[4..8].copy_from_slice(&(total_len as u32).to_le_bytes());
    xsdt[8] = 0x01;

    xsdt[10..16].copy_from_slice(b"FERRUM");
    xsdt[16..24].copy_from_slice(b"FERRUM  ");
    xsdt[24..28].copy_from_slice(&1u32.to_le_bytes());
    xsdt[28..32].copy_from_slice(b"FERR");
    xsdt[32..36].copy_from_slice(&1u32.to_le_bytes());

    for (i, &addr) in table_addrs.iter().enumerate() {
        let offset = 36 + i * 8;
        xsdt[offset..offset + 8].copy_from_slice(&addr.to_le_bytes());
    }

    xsdt[9] = checksum(&xsdt);

    Binary::new(xsdt, 0xE00E0)
}

fn checksum(data: &[u8]) -> u8 {
    let sum = data.iter().fold(0u8, |acc, &b| acc.wrapping_add(b));
    sum.wrapping_neg()
}
