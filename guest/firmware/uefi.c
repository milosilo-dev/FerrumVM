#include "headers/uefi/uefi.h"
#include "headers/uefi/crc32.h"
#include "headers/uefi/config_table.h"
#include "headers/uefi/stip.h"
#include "mem/heap.c"

#define STUB(name, ret) \
    static EFI_STATUS EFIAPI stub_##name( \
        void* a, void* b, void* c, void* d) { \
        serial_puts("[STUB] " #name "\n"); \
        return ret; \
    }

static EFI_STATUS EFIAPI stub_Null() {
    serial_puts("[STUB] NULL service called — halting\n");
    for (;;) __asm__("hlt");
}

// ── con out ───────────────────────────────────────────────────────

static EFI_STATUS EFIAPI efi_output_string(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *This,
    uint16_t *String
) {
    serial_puts("[STUB] STOP");
    while (*String) {
        char c = (char)(*String & 0xFF);
        if (c == '\n') serial_putc('\r');
        serial_putc(c);
        String++;
    }
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_Stall(UINTN microseconds) {
    serial_puts("[STUB] Stall us=");
    serial_putx(microseconds);
    // print return address so you know who's calling
    uint64_t ra;
    __asm__("mov 8(%%rbp), %0" : "=r"(ra));
    serial_puts(" caller=");
    serial_putx(ra);
    serial_puts("\n");
    return EFI_SUCCESS;
}

STUB(ConReset,        EFI_SUCCESS)
STUB(QueryMode,       EFI_SUCCESS)
STUB(SetMode,         EFI_SUCCESS)
STUB(SetAttribute,    EFI_SUCCESS)
STUB(ClearScreen,     EFI_SUCCESS)
STUB(SetCursorPosition,     EFI_SUCCESS)
STUB(EnableCursor,     EFI_SUCCESS)

static SIMPLE_TEXT_OUTPUT_MODE gConOutMode = {
    .MaxMode       = 1,
    .Mode          = 0,
    .Attribute     = 0,
    .CursorColumn  = 0,
    .CursorRow     = 0,
    .CursorVisible = 1,
};

static EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL gConOut = {
    .Reset        = (void*)stub_ConReset,
    .OutputString = efi_output_string,
    .QueryMode    = (void*)stub_QueryMode,
    .SetMode      = (void*)stub_SetMode,
    .SetAttribute = (void*)stub_SetAttribute,
    .ClearScreen  = (void*)stub_ClearScreen,
    .SetCursorPosition  = (void*)stub_SetCursorPosition,
    .EnableCursor       = (void*)stub_EnableCursor,
    .Mode               = &gConOutMode,
};

// Heap mem

static EFI_STATUS EFIAPI efi_AllocatePool(
    EFI_MEMORY_TYPE type,
    UINTN size,
    VOID **out
) {
    serial_puts("AllocatePool type=");
    serial_putx(type);
    serial_puts(" size=");
    serial_putx(size);
    serial_puts(" allocate pool out arg=");
    serial_putx((uint32_t)*out);
    serial_puts("\n");

    void* ptr = malloc(size);
    if (!ptr) return EFI_OUT_OF_RESOURCES;
    memset(ptr, 0, size);
    uint64_t ra;
    __asm__("mov 8(%%rbp), %0" : "=r"(ra));
    *out = ptr;

    uint64_t rsp;
    __asm__ volatile("mov %%rsp, %0" : "=r"(rsp));
    uint64_t retaddr = *(uint64_t*)rsp;

    serial_puts("AllocatePool result=");
    serial_putx((uint64_t)ptr);
    serial_puts("\n");

    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_AllocatePages(
    EFI_ALLOCATE_TYPE type,
    EFI_MEMORY_TYPE memory_type,
    UINTN pages,
    EFI_PHYSICAL_ADDRESS *memory
) {
    serial_puts("AllocatePages type=");
    serial_putx(type);
    serial_puts(" memory_type=");
    serial_putx(memory_type);
    serial_puts(" pages=");
    serial_putx(pages);
    serial_puts(" requested_addr=");
    serial_putx(*memory);
    serial_puts("\n");

    void* ptr = malloc(pages * 4096);
    if (!ptr) return EFI_OUT_OF_RESOURCES;
    memset(ptr, 0, pages * 4096);

    serial_puts("AllocatePages result=");
    serial_putx((uint64_t)ptr);
    serial_puts("\n");

    *memory = (EFI_PHYSICAL_ADDRESS)ptr;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_LocateProtocol(
    EFI_GUID *guid, VOID *reg, VOID **iface
) {
    serial_puts("[STUB] LocateProtocol {");
    serial_putx(guid->Data1); serial_puts("-");
    serial_putx(guid->Data2); serial_puts("-");
    serial_putx(guid->Data3); serial_puts("}\n");
    *iface = NULL;
    return EFI_NOT_FOUND;
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
STUB(SetWatchdogTimer,              EFI_SUCCESS)
STUB(ConnectController,             EFI_NOT_FOUND)
STUB(DisconnectController,          EFI_SUCCESS)
STUB(OpenProtocol,                  EFI_NOT_FOUND)
STUB(CloseProtocol,                 EFI_SUCCESS)
STUB(OpenProtocolInformation,       EFI_SUCCESS)
STUB(ProtocolsPerHandle,            EFI_SUCCESS)
STUB(LocateHandleBuffer,            EFI_NOT_FOUND)
STUB(InstallMultipleProtocolInterfaces,   EFI_SUCCESS)
STUB(UninstallMultipleProtocolInterfaces, EFI_SUCCESS)
STUB(CalculateCrc32,                EFI_SUCCESS)

static VOID EFIAPI stub_CopyMem(VOID *dst, VOID *src, UINTN len) { memcpy(dst, src, len); }
static VOID EFIAPI stub_SetMem(VOID *buf, UINTN size, UINT8 val) { memset(buf, val, size); }

static EFI_STATUS EFIAPI stub_Exit(EFI_HANDLE img, EFI_STATUS status, UINTN size, CHAR16 *data) {
    serial_puts("[STUB] Exit — halting\n");
    for (;;) __asm__("hlt");
}

static EFI_STATUS EFIAPI efi_LocateHandleBuffer(
    EFI_LOCATE_SEARCH_TYPE type,
    EFI_GUID *guid,
    VOID *key,
    UINTN *count,
    EFI_HANDLE **buf
) {
    serial_puts("[STUB] LocateHandleBuffer\n");
    if (count) *count = 0;
    if (buf)   *buf   = NULL;
    return EFI_NOT_FOUND;
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
    .LocateHandleBuffer                 = (void*)efi_LocateHandleBuffer,
    .LocateProtocol                     = (void*)efi_LocateProtocol,
    .InstallMultipleProtocolInterfaces  = (void*)stub_InstallMultipleProtocolInterfaces,
    .UninstallMultipleProtocolInterfaces= (void*)stub_UninstallMultipleProtocolInterfaces,
    .CalculateCrc32                     = (void*)stub_CalculateCrc32,
    .CopyMem                            = stub_CopyMem,
    .SetMem                             = stub_SetMem,
    .CreateEventEx                      = (void*)stub_CreateEvent, // reuse
};

// ── runtime services table ────────────────────────────────────────

STUB(GetTime,                EFI_UNSUPPORTED)
STUB(SetTime,                EFI_UNSUPPORTED)
STUB(GetWakeupTime,          EFI_UNSUPPORTED)
STUB(SetWakeupTime,          EFI_UNSUPPORTED)
STUB(SetVirtualAddressMap,   EFI_UNSUPPORTED)
STUB(ConvertPointer,         EFI_UNSUPPORTED)
STUB(GetVariable,            EFI_NOT_FOUND)
STUB(GetNextVariableName,    EFI_NOT_FOUND)
STUB(SetVariable,            EFI_UNSUPPORTED)
STUB(GetNextHighMonotonicCount, EFI_UNSUPPORTED)
STUB(ResetSystem,            EFI_SUCCESS)
STUB(UpdateCapsule,          EFI_UNSUPPORTED)
STUB(QueryCapsuleCapabilities, EFI_UNSUPPORTED)
STUB(QueryVariableInfo,      EFI_UNSUPPORTED)

static EFI_RUNTIME_SERVICES gRuntimeServices = {
    .Hdr = {
        .Signature  = EFI_RUNTIME_SERVICES_SIGNATURE,
        .Revision   = EFI_RUNTIME_SERVICES_REVISION,
        .HeaderSize = sizeof(EFI_RUNTIME_SERVICES),
        .CRC32      = 0,
        .Reserved   = 0,
    },
    .GetTime                    = (void*)stub_GetTime,
    .SetTime                    = (void*)stub_SetTime,
    .GetWakeupTime              = (void*)stub_GetWakeupTime,
    .SetWakeupTime              = (void*)stub_SetWakeupTime,
    .SetVirtualAddressMap       = (void*)stub_SetVirtualAddressMap,
    .ConvertPointer             = (void*)stub_ConvertPointer,
    .GetVariable                = (void*)stub_GetVariable,
    .GetNextVariableName        = (void*)stub_GetNextVariableName,
    .SetVariable                = (void*)stub_SetVariable,
    .GetNextHighMonotonicCount  = (void*)stub_GetNextHighMonotonicCount,
    .ResetSystem                = (void*)stub_ResetSystem,
    .UpdateCapsule              = (void*)stub_UpdateCapsule,
    .QueryCapsuleCapabilities   = (void*)stub_QueryCapsuleCapabilities,
    .QueryVariableInfo          = (void*)stub_QueryVariableInfo,
};

static EFI_CONFIGURATION_TABLE gConfigTables[2];

// ── system table ──────────────────────────────────────────────────

void format_config_table() {
    gConfigTables[0].VendorGuid = (EFI_GUID)ACPI_20_TABLE_GUID;
    gConfigTables[0].VendorTable = (void*)(0xE0000);

    gConfigTables[1].VendorGuid = (EFI_GUID)ACPI_TABLE_GUID;
    gConfigTables[1].VendorTable = (void*)(0xE0000);
}

static uint16_t gFirmwareVendor[] = { 'F','e','r','r','u','m', 0 };

void patch_null_stubs(void) {
    void** tbl = (void**)&gBootServices;
    // skip the header (40 bytes = 5 pointers)
    for (int i = 5; i < sizeof(EFI_BOOT_SERVICES)/8; i++) {
        if (tbl[i] == NULL)
            tbl[i] = stub_Null;
    }

    tbl = (void**)&gRuntimeServices;
    // skip the header (40 bytes = 5 pointers)
    for (int i = 5; i < sizeof(EFI_RUNTIME_SERVICES)/8; i++) {
        if (tbl[i] == NULL)
            tbl[i] = stub_Null;
    }
}

void format_system_table(EFI_SYSTEM_TABLE *st) {
    st->Hdr.Signature  = EFI_SYSTEM_TABLE_SIGNATURE;
    st->Hdr.Revision   = (2 << 16) | 70;
    st->Hdr.HeaderSize = sizeof(EFI_SYSTEM_TABLE);
    st->Hdr.CRC32      = 0;
    st->Hdr.Reserved   = 0;

    st->FirmwareVendor   = gFirmwareVendor;
    st->FirmwareRevision = 1;

    st->ConsoleInHandle     = (EFI_HANDLE)1;
    st->ConIn               = &gConIn;
    st->ConsoleOutHandle    = (EFI_HANDLE)2;
    st->ConOut              = &gConOut;
    st->StandardErrorHandle = (EFI_HANDLE)3;
    st->StdErr              = &gConOut;

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