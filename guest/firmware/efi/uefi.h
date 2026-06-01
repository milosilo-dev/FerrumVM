#pragma once
#include "../headers/uefi/uefi.h"
#include "../headers/uefi/image_handle.h"
#include "../headers/uefi/config_table.h"
#include "blockio.h"

extern EFI_GUID gEfiBlockIoProtocolGuid;
extern EFI_BLOCK_IO gBlockIo;
extern EFI_HANDLE gDiskHandle;
extern EFI_LOADED_IMAGE_PROTOCOL* gLoadedImageInstance;

void format_system_table(EFI_SYSTEM_TABLE *st);
void patch_null_stubs(void);
void format_handle_data(EFI_IMAGE_HANDLE_DATA* handle_data, EFI_SYSTEM_TABLE *st, uint32_t image_size, uint8_t* load_base);
void efi_init(EFI_SYSTEM_TABLE *st, EFI_HANDLE image_handle);
void efi_register_protocol(EFI_HANDLE handle, EFI_GUID *guid, void *iface);
