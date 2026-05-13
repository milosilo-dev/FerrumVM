#include "headers/uefi/uefi.h"
#include "headers/uefi/crc32.h"
#include "headers/uefi/config_table.h"
#include "mem/heap.c"

#define STUB(name, ret) \
    static EFI_STATUS EFIAPI stub_##name() { \
        serial_puts("[STUB] " #name "\n"); \
        return ret; \
    }

// ── con out ───────────────────────────────────────────────────────

static EFI_STATUS EFIAPI efi_output_string(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *This,
    uint16_t *String
) {
    while (*String) {
        char c = (char)(*String & 0xFF);
        if (c == '\n') serial_putc('\r');
        serial_putc(c);
        String++;
    }
    return EFI_SUCCESS;
}

static EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL gConOut = {
    .Reset        = NULL,
    .OutputString = efi_output_string,
    .QueryMode    = NULL,
    .SetMode      = NULL,
    .SetAttribute = NULL,
    .ClearScreen  = NULL,
};

// Heap mem

static EFI_STATUS EFIAPI efi_AllocatePool(
    EFI_MEMORY_TYPE type,
    UINTN size,
    VOID **out
) {
    void* ptr = malloc(size);
    if (!ptr) return EFI_OUT_OF_RESOURCES;
    *out = ptr;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_AllocatePages(
    EFI_ALLOCATE_TYPE type,
    EFI_MEMORY_TYPE memory_type,
    UINTN pages,
    EFI_PHYSICAL_ADDRESS *memory
) {
    void* ptr = malloc(pages * 4096);
    if (!ptr) return EFI_OUT_OF_RESOURCES;
    *memory = (EFI_PHYSICAL_ADDRESS)ptr;
    return EFI_SUCCESS;
}

// ── boot service stubs ────────────────────────────────────────────

STUB(RaiseTPL,                      EFI_SUCCESS)
STUB(RestoreTPL,                    EFI_SUCCESS)
STUB(FreePages,                     EFI_SUCCESS)
STUB(GetMemoryMap,                  EFI_BUFFER_TOO_SMALL)
STUB(FreePool,                      EFI_SUCCESS)
STUB(CreateEvent,                   EFI_SUCCESS)
STUB(SetTimer,                      EFI_SUCCESS)
STUB(WaitForEvent,                  EFI_SUCCESS)
STUB(SignalEvent,                   EFI_SUCCESS)
STUB(CloseEvent,                    EFI_SUCCESS)
STUB(CheckEvent,                    EFI_NOT_READY)
STUB(InstallProtocolInterface,      EFI_SUCCESS)
STUB(ReinstallProtocolInterface,    EFI_SUCCESS)
STUB(UninstallProtocolInterface,    EFI_SUCCESS)
STUB(HandleProtocol,                EFI_NOT_FOUND)
STUB(RegisterProtocolNotify,        EFI_SUCCESS)
STUB(LocateHandle,                  EFI_NOT_FOUND)
STUB(LocateDevicePath,              EFI_NOT_FOUND)
STUB(InstallConfigurationTable,     EFI_SUCCESS)
STUB(LoadImage,                     EFI_UNSUPPORTED)
STUB(StartImage,                    EFI_UNSUPPORTED)
STUB(UnloadImage,                   EFI_UNSUPPORTED)
STUB(ExitBootServices,              EFI_SUCCESS)
STUB(GetNextMonotonicCount,         EFI_SUCCESS)
STUB(Stall,                         EFI_SUCCESS)
STUB(SetWatchdogTimer,              EFI_SUCCESS)
STUB(ConnectController,             EFI_NOT_FOUND)
STUB(DisconnectController,          EFI_SUCCESS)
STUB(OpenProtocol,                  EFI_NOT_FOUND)
STUB(CloseProtocol,                 EFI_SUCCESS)
STUB(OpenProtocolInformation,       EFI_SUCCESS)
STUB(ProtocolsPerHandle,            EFI_SUCCESS)
STUB(LocateHandleBuffer,            EFI_NOT_FOUND)
STUB(LocateProtocol,                EFI_NOT_FOUND)
STUB(InstallMultipleProtocolInterfaces,   EFI_SUCCESS)
STUB(UninstallMultipleProtocolInterfaces, EFI_SUCCESS)
STUB(CalculateCrc32,                EFI_SUCCESS)

static VOID EFIAPI stub_CopyMem(VOID *dst, VOID *src, UINTN len) { memcpy(dst, src, len); }
static VOID EFIAPI stub_SetMem(VOID *buf, UINTN size, UINT8 val) { memset(buf, val, size); }

static EFI_STATUS EFIAPI stub_Exit(EFI_HANDLE img, EFI_STATUS status, UINTN size, CHAR16 *data) {
    serial_puts("[STUB] Exit — halting\n");
    for (;;) __asm__("hlt");
}

// ── boot services table ───────────────────────────────────────────

static EFI_BOOT_SERVICES gBootServices = {
    .Hdr = {
        .Signature  = EFI_BOOT_SERVICES_SIGNATURE,
        .Revision   = EFI_BOOT_SERVICES_REVISION,
        .HeaderSize = sizeof(EFI_BOOT_SERVICES),
        .CRC32      = 0,
        .Reserved   = 0
    },
    .RaiseTPL                           = (void*)stub_RaiseTPL,
    .RestoreTPL                         = (void*)stub_RestoreTPL,
    .AllocatePages                      = (void*)efi_AllocatePages,
    .FreePages                          = (void*)stub_FreePages,
    .GetMemoryMap                       = (void*)stub_GetMemoryMap,
    .AllocatePool                       = (void*)efi_AllocatePool,
    .FreePool                           = (void*)stub_FreePool,
    .CreateEvent                        = (void*)stub_CreateEvent,
    .SetTimer                           = (void*)stub_SetTimer,
    .WaitForEvent                       = (void*)stub_WaitForEvent,
    .SignalEvent                        = (void*)stub_SignalEvent,
    .CloseEvent                         = (void*)stub_CloseEvent,
    .CheckEvent                         = (void*)stub_CheckEvent,
    .InstallProtocolInterface           = (void*)stub_InstallProtocolInterface,
    .ReinstallProtocolInterface         = (void*)stub_ReinstallProtocolInterface,
    .UninstallProtocolInterface         = (void*)stub_UninstallProtocolInterface,
    .HandleProtocol                     = (void*)stub_HandleProtocol,
    .Reserved                           = NULL,
    .RegisterProtocolNotify             = (void*)stub_RegisterProtocolNotify,
    .LocateHandle                       = (void*)stub_LocateHandle,
    .LocateDevicePath                   = (void*)stub_LocateDevicePath,
    .InstallConfigurationTable          = (void*)stub_InstallConfigurationTable,
    .LoadImage                          = (void*)stub_LoadImage,
    .StartImage                         = (void*)stub_StartImage,
    .Exit                               = stub_Exit,
    .UnloadImage                        = (void*)stub_UnloadImage,
    .ExitBootServices                   = (void*)stub_ExitBootServices,
    .GetNextMonotonicCount              = (void*)stub_GetNextMonotonicCount,
    .Stall                              = (void*)stub_Stall,
    .SetWatchdogTimer                   = (void*)stub_SetWatchdogTimer,
    .ConnectController                  = (void*)stub_ConnectController,
    .DisconnectController               = (void*)stub_DisconnectController,
    .OpenProtocol                       = (void*)stub_OpenProtocol,
    .CloseProtocol                      = (void*)stub_CloseProtocol,
    .OpenProtocolInformation            = (void*)stub_OpenProtocolInformation,
    .ProtocolsPerHandle                 = (void*)stub_ProtocolsPerHandle,
    .LocateHandleBuffer                 = (void*)stub_LocateHandleBuffer,
    .LocateProtocol                     = (void*)stub_LocateProtocol,
    .InstallMultipleProtocolInterfaces  = (void*)stub_InstallMultipleProtocolInterfaces,
    .UninstallMultipleProtocolInterfaces= (void*)stub_UninstallMultipleProtocolInterfaces,
    .CalculateCrc32                     = (void*)stub_CalculateCrc32,
    .CopyMem                            = stub_CopyMem,
    .SetMem                             = stub_SetMem,
    .CreateEventEx                      = (void*)stub_CreateEvent, // reuse
};

// ── runtime services table ────────────────────────────────────────

static EFI_RUNTIME_SERVICES gRuntimeServices = {
    .Hdr = {
        .Signature  = EFI_RUNTIME_SERVICES_SIGNATURE,
        .Revision   = EFI_RUNTIME_SERVICES_REVISION,
        .HeaderSize = sizeof(EFI_RUNTIME_SERVICES),
        .CRC32      = 0,
        .Reserved   = 0,
    },
    // all NULL for now — Limine doesn't call runtime services before ExitBootServices
};

static EFI_CONFIGURATION_TABLE gConfigTable[2];

// ── system table ──────────────────────────────────────────────────

void format_config_table() {
    gConfigTables[0].VendorGuid = (EFI_GUID)ACPI_20_TABLE_GUID;
    gConfigTables[0].VendorTable = rsdp_address;

    config_tables[1].VendorGuid = (EFI_GUID)ACPI_TABLE_GUID;
    config_tables[1].VendorTable = rsdp_address;
}

static uint16_t gFirmwareVendor[] = { 'F','e','r','r','u','m', 0 };

void format_system_table(EFI_SYSTEM_TABLE *st) {
    st->Hdr.Signature  = EFI_SYSTEM_TABLE_SIGNATURE;
    st->Hdr.Revision   = (2 << 16) | 70;
    st->Hdr.HeaderSize = sizeof(EFI_SYSTEM_TABLE);
    st->Hdr.CRC32      = 0;
    st->Hdr.Reserved   = 0;

    st->FirmwareVendor   = gFirmwareVendor;
    st->FirmwareRevision = 1;

    st->ConsoleInHandle     = NULL;
    st->ConIn               = NULL;
    st->ConsoleOutHandle    = NULL;
    st->ConOut              = &gConOut;
    st->StandardErrorHandle = NULL;
    st->StdErr              = NULL;

    gBootServices.Hdr.CRC32 = crc32((uint8_t*)&gBootServices, 
            gBootServices.Hdr.HeaderSize);
    st->BootServices    = &gBootServices;

    gRuntimeServices.Hdr.CRC32 = crc32((uint8_t*)&gRuntimeServices, 
        gRuntimeServices.Hdr.HeaderSize);
    st->RuntimeServices = &gRuntimeServices;

    st->NumberOfTableEntries = 0;
    st->ConfigurationTable   = NULL;

    st->Hdr.CRC32 = crc32((uint8_t*)st, st->Hdr.HeaderSize);
}

void format_handle_data(EFI_IMAGE_HANDLE_DATA* handle_data, EFI_SYSTEM_TABLE *st, uint32_t image_size, uint8_t* load_base) {
    handle_data->loaded_image.ImageBase   = load_base;
    handle_data->loaded_image.ImageSize   = image_size;
    handle_data->loaded_image.SystemTable = st;
}