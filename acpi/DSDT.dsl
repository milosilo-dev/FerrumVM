DefinitionBlock (
    "DSDT.aml",
    "DSDT",
    2,
    "FERRUM",
    "VM_DSDT",
    0x00001000
)
{
    Scope (\_SB)
    {
        // 1. Define the MMIO Region Container
        // This acts as the bus for our VirtIO devices
        Device (VIRT)
        {
            // Valid EISA ID: 3 letters + 4 hex digits
            Name (_HID, EisaId ("VIR0001")) 
            Name (_CID, EisaId ("PNP0A06"))
            Name (_UID, 0)

            // Define the Memory Range for VirtIO MMIO
            // Update to match your VMM
            Method (_CRS, 0, NotSerialized)
            {
                Return (ResourceTemplate ()
                {
                    // Memory32Fixed: ReadWrite, Base, Length
                    Memory32Fixed (ReadWrite, 0x20000000, 0x10000)
                })
            }

            // 2. Define VirtIO Devices
            // Each device needs a unique _UID and an _ADR (offset)
            
            // VirtIO RNG
            Device (RNG0)
            {
                Name (_UID, 1)
                Name (_ADR, 0x00000000) // Offset 0x0
                
                Method (_STA, 0, NotSerialized) { Return (0x0F) }
            }

            // VirtIO Counter
            Device (CNT0)
            {
                Name (_UID, 2)
                Name (_ADR, 0x00001000) // Offset 0x1000
                
                Method (_STA, 0, NotSerialized) { Return (0x0F) }
            }

            // VirtIO Block (Disk)
            Device (DISK)
            {
                Name (_UID, 3)
                Name (_ADR, 0x00002000) // Offset 0x2000
                
                Method (_STA, 0, NotSerialized) { Return (0x0F) }
            }
        }
    }
}