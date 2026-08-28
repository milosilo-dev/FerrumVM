use crate::machine_config::binary::Binary;

pub fn build_madt(vcpu_num: u8) -> Binary {
    let mut madt = vec![0u8; (44 + vcpu_num * 8 + 12) as usize];

    // Header (36 bytes)
    madt[0..4].copy_from_slice(b"APIC"); // Signature
    madt[4..8].copy_from_slice(&((44 + vcpu_num * 8 + 12) as u32).to_le_bytes()); // Length
    madt[8] = 2; // Revision (ACPI 3.0)
    madt[9] = 0; // Checksum (calculated later)
    madt[10..16].copy_from_slice(b"FERRUM"); // OEM ID
    madt[16..24].copy_from_slice(b"FVM_MADT"); // OEM Table ID
    madt[24..28].copy_from_slice(&(1u32).to_le_bytes()); // OEM Revision
    madt[28..32].copy_from_slice(b"FVM "); // Creator ID
    madt[32..36].copy_from_slice(&(1u32).to_le_bytes()); // Creator Revision

    // Local ACPI address
    madt[36..40].copy_from_slice(&(0xFEE00000u32).to_le_bytes());
    // Flags
    madt[40..44].copy_from_slice(&(1u32).to_le_bytes());
    
    // VCPU's
    let mut offset = 44;
    for vcpu_id in 0..vcpu_num{
        madt[offset] = 0; // Type
        madt[offset + 1] = 8; // Length
        madt[offset + 2] = vcpu_id; // ACPI Processor UID
        madt[offset + 3] = vcpu_id; // ACPI ID
        madt[offset + 4..offset + 8].copy_from_slice(&(1u32).to_le_bytes()); // Flags
        offset += 8;
    }

    // IOAPIC
    madt[offset] = 1; // Type
    madt[offset + 1] = 12; // Length
    madt[offset + 2] = 0; // IOAPIC ID
    madt[offset + 3] = 0; // Reserved
    madt[offset + 4..offset + 8].copy_from_slice(&0xFEC00000u32.to_le_bytes()); // IOAPIC address
    madt[offset + 8..offset + 12].copy_from_slice(&0u32.to_le_bytes()); // GSI base

    let checksum = madt
        .iter()
        .fold(0u8, |sum, byte| {
            sum.wrapping_add(*byte)
        });

    madt[9] = checksum.wrapping_neg();

    Binary::new(madt, 0xE0500)
}