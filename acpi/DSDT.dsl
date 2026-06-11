/*
 * Intel ACPI Component Architecture
 * AML/ASL+ Disassembler version 20251212 (64-bit version)
 * Copyright (c) 2000 - 2025 Intel Corporation
 * 
 * Disassembling to symbolic ASL+ operators
 *
 * Disassembly of /home/miles/Data/FerrumVM/acpi/DSDT.aml
 *
 * Original Table Header:
 *     Signature        "DSDT"
 *     Length           0x000000CD (205)
 *     Revision         0x02
 *     Checksum         0x75
 *     OEM ID           "FERRUM"
 *     OEM Table ID     "VM_DSDT"
 *     OEM Revision     0x00001000 (4096)
 *     Compiler ID      "INTL"
 *     Compiler Version 0x20251212 (539300370)
 */
DefinitionBlock ("", "DSDT", 2, "FERRUM", "VM_DSDT", 0x00001000)
{
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
                    Memory32Fixed (ReadWrite,
                        0x20000000,         // Address Base
                        0x00010000,         // Address Length
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

            Device (NET){
                Name (_UID, 0x04)
                Name (_ADR, 0x3000)
                Method (_STA, 0, NotSerialized){
                    Return (0x0F)
                }
            }
        }
    }
}

