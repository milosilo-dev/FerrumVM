#pragma once
#include "../headers/uefi/uefi.h"

#define MEDIA_DEVICE_PATH              0x04
#define MEDIA_HARDDRIVE_DP             0x01

#define END_DEVICE_PATH_TYPE           0x7F
#define END_ENTIRE_DEVICE_PATH_SUBTYPE 0xFF

typedef struct {
    EFI_DEVICE_PATH_PROTOCOL Header;

    uint32_t PartitionNumber;
    uint64_t PartitionStart;
    uint64_t PartitionSize;

    uint8_t  Signature[16];
    uint8_t  MBRType;
    uint8_t  SignatureType;
} HARDDRIVE_DEVICE_PATH;

typedef struct {
    HARDDRIVE_DEVICE_PATH Hd;
    EFI_DEVICE_PATH_PROTOCOL End;
} DISK_PATH;