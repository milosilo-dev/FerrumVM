// Fully generate a new FADT binary programiaticly

use crate::machine_config::binary::{Binary};

pub fn build_fadt(dsdt_addr: u64) -> Binary {
    let mut fadt = vec![0u8; 240]; // FADT length (ACPI 5.0)

    // Header (36 bytes)
    fadt[0..4].copy_from_slice(b"FACP"); // Signature
    fadt[4..8].copy_from_slice(&(240u32).to_le_bytes()); // Length
    fadt[8] = 2; // Revision
    fadt[9] = 0; // Checksum (calculated later)
    fadt[10..16].copy_from_slice(b"FERRUM"); // OEM ID
    fadt[16..24].copy_from_slice(b"FVM_FADT"); // OEM Table ID
    fadt[24..28].copy_from_slice(&(1u32).to_le_bytes()); // OEM Revision
    fadt[28..32].copy_from_slice(b"FVM "); // Creator ID
    fadt[32..36].copy_from_slice(&(1u32).to_le_bytes()); // Creator Revision

    // FADT Fields (starting at offset 36)
    // Firmware Control (FACS) - 0 for KVM
    // DSDT Address (32-bit) - Set to 0, use X_DSDT for 64-bit
    // ... (Set other fields to 0 as needed)

    // X_DSDT (64-bit address) - CRITICAL: Point to your DSDT.aml
    fadt[108..116].copy_from_slice(&dsdt_addr.to_le_bytes()); // X_DSDT

    // Calculate Checksum
    let mut sum: u8 = 0;
    for byte in &fadt {
        sum = sum.wrapping_add(*byte);
    }
    fadt[9] = sum.wrapping_neg(); // Set checksum so total sum is 0

    Binary::new(
        fadt,
        0xE0400,
    )
}