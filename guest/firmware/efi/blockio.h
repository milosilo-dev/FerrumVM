#pragma once
#include "../headers/uefi/uefi.h"

#define EFI_BLOCK_IO_PROTOCOL_REVISION 0x00010000

typedef struct _EFI_BLOCK_IO EFI_BLOCK_IO;
typedef UINT64 EFI_LBA;

typedef struct {
    UINT32   MediaId;
    BOOLEAN  RemovableMedia;
    BOOLEAN  MediaPresent;
    BOOLEAN  LogicalPartition;
    BOOLEAN  ReadOnly;
    BOOLEAN  WriteCaching;
    UINT32   BlockSize;
    UINT32   IoAlign;
    EFI_LBA  LastBlock;
} EFI_BLOCK_IO_MEDIA;

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_RESET)(
    EFI_BLOCK_IO *This, BOOLEAN ExtendedVerification);

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_READ)(
    EFI_BLOCK_IO *This, UINT32 MediaId, EFI_LBA LBA,
    UINTN BufferSize, VOID *Buffer);

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_WRITE)(
    EFI_BLOCK_IO *This, UINT32 MediaId, EFI_LBA LBA,
    UINTN BufferSize, VOID *Buffer);

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_FLUSH)(EFI_BLOCK_IO*);

struct _EFI_BLOCK_IO {
    UINT64             Revision;
    EFI_BLOCK_IO_MEDIA *Media;
    EFI_BLOCK_RESET    Reset;
    EFI_BLOCK_READ     ReadBlocks;
    EFI_BLOCK_WRITE    WriteBlocks;
    EFI_BLOCK_FLUSH    FlushBlocks;
};