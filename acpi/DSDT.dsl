/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of acpi/DSDT.aml
 *
 * Original Table Header:
 *     Signature        "DSDT"
 *     Length           0x0000012D (301)
 *     Revision         0x02
 *     Checksum         0x98
 *     OEM ID           "FERRUM"
 *     OEM Table ID     "VM_DSDT"
 *     OEM Revision     0x00001000 (4096)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20251212 (539300370)
 */
DefinitionBlock ("", "DSDT", 2, "FERRUM", "FVM_DSDT", 0x00001000)
{
    Scope (_PR)
    {
        Processor (CPU0, 0x00, 0x0000B010, 0x06){}
        Processor (CPU1, 0x01, 0x0000B010, 0x06){}
    }

    Scope (_SB)
    {
        Device (VIRT)
        {
            Name (_HID, "VIRT0001")  // _HID: Hardware ID
            Name (_CID, "PNP0A06" /* Generic Container Device */)  // _CID: Compatible ID
            Name (_UID, Zero)  // _UID: Unique ID
            Method (_CRS, 0, NotSerialized)  // _CRS: Current Resource Settings
            {
                Return (ResourceTemplate ()
                {
                    QWordMemory (
                        ResourceConsumer,   // resource usage
                        PosDecode,           // decode
                        MinFixed,            // is min fixed
                        MaxFixed,            // is max fixed
                        Cacheable,           // cacheable
                        ReadWrite,           // read/write
                        0x0000000000000000,  // granularity
                        0x0000000400000000,  // range minimum
                        0x000000040000FFFF,  // range maximum
                        0x0000000000000000,  // translation
                        0x0000000000010000,  // range length
                        )
                })
            }

            Device (RNG0)
            {
                Name (_UID, One)  // _UID: Unique ID
                Name (_ADR, Zero)  // _ADR: Address
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    Return (0x0F)
                }
            }

            Device (CNT0)
            {
                Name (_UID, 0x02)  // _UID: Unique ID
                Name (_ADR, 0x1000)  // _ADR: Address
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    Return (0x0F)
                }
            }

            Device (DISK)
            {
                Name (_UID, 0x03)  // _UID: Unique ID
                Name (_ADR, 0x2000)  // _ADR: Address
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    Return (0x0F)
                }
            }

            Device (NET)
            {
                Name (_UID, 0x04)  // _UID: Unique ID
                Name (_ADR, 0x3000)  // _ADR: Address
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    Return (0x0F)
                }
            }

            Device (FS)
            {
                Name (_UID, 0x05)  // _UID: Unique ID
                Name (_ADR, 0x4000)  // _ADR: Address
                Method (_STA, 0, NotSerialized)  // _STA: Status
                {
                    Return (0x0F)
                }
            }
        }
    }
}

