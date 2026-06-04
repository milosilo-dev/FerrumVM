// disk handle — distinct from the fake handle
#include "blockio.h"

static int gDiskHandleData;
EFI_HANDLE gDiskHandle = (EFI_HANDLE)&gDiskHandleData;

EFI_GUID gEfiBlockIoProtocolGuid = {
    0x964E5B21, 0x6459, 0x11D2,
    {0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

static EFI_STATUS EFIAPI disk_ReadBlocks(
    EFI_BLOCK_IO *this,
    UINT32 MediaId,
    EFI_LBA lba,
    UINTN BufferSize,
    VOID *Buffer
) {
    serial_puts("[BLOCKIO] Read Blocks lba=0x");
    serial_putx(lba);
    serial_puts(" buffer_size=0x");
    serial_putx(BufferSize);
    int status = virtio_blk_read(lba, BufferSize, Buffer);
    serial_puts(" buffer (first 16 uint32_t's)=[0x");
    for (int i = 0; i < 16; i++) {
        serial_putx(((uint32_t *)Buffer)[i]);
        if (i != 15){
            serial_puts(",0x");
        }
    }
    serial_puts("] status=0x");
    serial_putx(status);
    serial_puts("\n");
    return status == 0 ? EFI_SUCCESS : EFI_DEVICE_ERROR;
}

EFI_BLOCK_IO_MEDIA gDiskMedia = {
    .MediaId          = 1,
    .RemovableMedia   = 0,
    .MediaPresent     = 1,
    .LogicalPartition = 0,
    .ReadOnly         = 1,
    .WriteCaching     = 0,
    .BlockSize        = 512,
    .IoAlign          = 0,
};

static EFI_STATUS EFIAPI stub_Reset(EFI_BLOCK_IO *This, BOOLEAN ExtendedVerification) {
    serial_puts("[STUB] Reset\n");
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_WriteBlock(    EFI_BLOCK_IO *This, UINT32 MediaId, EFI_LBA LBA,
    UINTN BufferSize, VOID *Buffer) 
{
    serial_puts("[STUB] WriteBlock\n");
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_FlushBlocks(EFI_BLOCK_IO *this) {
    serial_puts("[STUB] FlushBlocks\n");
    return EFI_SUCCESS;
}

EFI_BLOCK_IO gBlockIo = {
    .Revision    = EFI_BLOCK_IO_PROTOCOL_REVISION,
    .Media       = &gDiskMedia,
    .Reset       = stub_Reset,
    .ReadBlocks  = disk_ReadBlocks,
    .WriteBlocks = stub_WriteBlock,
    .FlushBlocks = stub_FlushBlocks,
};