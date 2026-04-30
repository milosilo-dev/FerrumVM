#pragma once
#include <stdint.h>

typedef uint64_t EFI_STATUS;
typedef void*    EFI_HANDLE;
typedef void*    EFI_EVENT;
typedef uint64_t EFI_PHYSICAL_ADDRESS;
typedef uint64_t EFI_VIRTUAL_ADDRESS;

#define EFI_SUCCESS 0
#define EFI_UNSUPPORTED  0x8000000000000003
#define EFI_NOT_FOUND    0x800000000000000E
#define EFI_BUFFER_TOO_SMALL 0x8000000000000005

typedef struct {
    uint32_t Data1;
    uint16_t Data2;
    uint16_t Data3;
    uint8_t  Data4[8];
} EFI_GUID;

typedef struct {
    uint64_t Signature;
    uint32_t Revision;
    uint32_t HeaderSize;
    uint32_t CRC32;
    uint32_t Reserved;
} EFI_TABLE_HEADER;

typedef struct {
    uint32_t Type;
    uint32_t Pad;
    EFI_PHYSICAL_ADDRESS PhysicalStart;
    EFI_VIRTUAL_ADDRESS  VirtualStart;
    uint64_t NumberOfPages;
    uint64_t Attribute;
} EFI_MEMORY_DESCRIPTOR;

#define EfiLoaderCode        1
#define EfiLoaderData        2
#define EfiBootServicesCode  3
#define EfiBootServicesData  4
#define EfiConventionalMemory 7
#define EfiACPIReclaimMemory  9
#define EfiMemoryMappedIO    11
#define EfiMemoryMappedIOPortSpace 12