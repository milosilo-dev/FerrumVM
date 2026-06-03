#pragma once
#include "../headers/uefi/uefi.h"
#include "../headers/uefi/image_handle.h"
#include "../headers/uefi/config_table.h"
#include "../headers/uefi/file_path.h"
#include "blockio.h"
#include "dev_path.h"
#include "sfsp.h"

extern EFI_GUID gEfiDevicePathProtocolGuid;
extern EFI_DEVICE_PATH_PROTOCOL gDevicePath;
extern DISK_PATH gDiskPath;

extern EFI_GUID gEfiSimpleFileSystemProtocolGuid;
extern EFI_SIMPLE_FILE_SYSTEM_PROTOCOL gSfsp;

extern EFI_GUID gEfiBlockIoProtocolGuid;
extern EFI_BLOCK_IO gBlockIo;
extern EFI_BLOCK_IO_MEDIA gDiskMedia;
extern EFI_HANDLE gDiskHandle;
extern EFI_LOADED_IMAGE_PROTOCOL* gLoadedImageInstance;

void format_system_table(EFI_SYSTEM_TABLE *st);
void patch_null_stubs(void);
void format_handle_data(EFI_IMAGE_HANDLE_DATA* handle_data, EFI_SYSTEM_TABLE *st, uint32_t image_size, uint8_t* load_base);
void efi_init(EFI_SYSTEM_TABLE *st, EFI_HANDLE image_handle);
void efi_register_protocol(EFI_HANDLE handle, EFI_GUID *guid, void *iface);
