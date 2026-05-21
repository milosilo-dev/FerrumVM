#include "headers/uefi/uefi.h"
#include "headers/serial.h"
#include "headers/uefi/crc32.h"
#include "headers/uefi/config_table.h"
#include "headers/uefi/stip.h"
#include "mem/heap.c"

#define TSC_MHZ 3000

#define STUB(name, ret) \
    static EFI_STATUS EFIAPI stub_##name( \
        void* a, void* b, void* c, void* d) { \
        serial_puts("[STUB] " #name "\n"); \
        return ret; \
    }

static EFI_STATUS EFIAPI stub_Null() {
    return EFI_SUCCESS;
}

static uint64_t map_key = 1;

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

static EFI_STATUS EFIAPI stub_Stall(UINTN microseconds) {
    serial_puts("[EFI] Stall us=");
    serial_putx(microseconds);
    serial_puts("\n");
    volatile uint64_t x = microseconds * 1000;

    while (x--)
        __asm__ volatile("pause");

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

// ── page allocator for efi_AllocatePages ────────────────────────────
// Uses the existing heap-based malloc (same as AllocatePool) so that
// allocated pages are safely inside the heap region (0x3000000-0x4000000)
// and never collide with firmware page-tables, memory-map data, or
// low-memory areas.
#define PG_LIMIT 0x20000000ULL  // end of our memory slot (512 MB)
static uint64_t pg_bump = 0x4000000ULL;  // start after heap (64 MB)

static EFI_STATUS EFIAPI efi_AllocatePool(
    EFI_MEMORY_TYPE type,
    UINTN size,
    VOID **out
) {
    serial_puts("[EFI] AlloactePool\n");
    void* ptr = malloc(size);
    if (!ptr) return EFI_OUT_OF_RESOURCES;
    memset(ptr, 0, size);
    *out = ptr;
    map_key++;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_AllocatePages(
    EFI_ALLOCATE_TYPE type,
    EFI_MEMORY_TYPE memory_type,
    UINTN pages,
    EFI_PHYSICAL_ADDRESS *memory
) {
    serial_puts("[EFI] AlloactePages\n");
    uint64_t size = pages * 4096;
    uint64_t addr;
    switch (type) {
    case AllocateAddress: {
        // Check that the requested address is within the memory slot and
        // outside the firmware's private low-memory region (< 4 MB).
        // The firmware uses 0x0–0x7FFFF for page tables, stage2, memmap, etc.
        // and 0x100000–0x1257000 for its main64 + PE loader.
        // Allow anything at or above 0x800000 (8 MB) as safe conventional RAM.
        addr = *memory;
        if (addr < 0x800000ULL || addr + size > PG_LIMIT)
            return EFI_OUT_OF_RESOURCES;
        break;
    }
    case AllocateMaxAddress: {
        uint64_t max = *memory;
        if (max < size) return EFI_OUT_OF_RESOURCES;
        addr = (max - size) & ~0xFFFULL;
        if (addr < pg_bump) addr = pg_bump;
        break;
    }
    case AllocateAnyPages:
    default:
        addr = pg_bump;
        pg_bump += size;
        if (addr + size > PG_LIMIT)
            return EFI_OUT_OF_RESOURCES;
        break;
    }
    map_key++;
    *memory = addr;
    memset((void*)(uintptr_t)addr, 0, size);
    return EFI_SUCCESS;
}

// ── real loaded-image protocol (set by format_PE before entry) ──
static EFI_LOADED_IMAGE_PROTOCOL* gLoadedImageInstance = NULL;

static int efi_guid_match(EFI_GUID* a, EFI_GUID* b) {
    return memcmp(a, b, sizeof(EFI_GUID)) == 0;
}

static EFI_GUID gEfiLoadedImageProtocolGuid2 = {
    0x5B1B31A1, 0x9562, 0x11D2,
    {0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

// ── simple handle database ─────────────────────────────────────────
#define MAX_PROTOCOLS 64
static struct {
    EFI_HANDLE handle;
    EFI_GUID   guid;
    void*      iface;
} gProtocolDB[MAX_PROTOCOLS];
static int gProtocolCount;

static EFI_STATUS EFIAPI efi_InstallProtocolInterface(
    EFI_HANDLE *Handle,
    EFI_GUID   *Protocol,
    VOID       *Interface,
    VOID       *DevicePath
) {
    serial_puts("[EFI] InstallProtocolInterface\n");
    if (Handle && *Handle == NULL)
        *Handle = (EFI_HANDLE)(uint64_t)4;
    if (Handle && Protocol && gProtocolCount < MAX_PROTOCOLS) {
        gProtocolDB[gProtocolCount].handle = *Handle;
        gProtocolDB[gProtocolCount].guid   = *Protocol;
        gProtocolDB[gProtocolCount].iface  = Interface;
        gProtocolCount++;
    }
    return EFI_SUCCESS;
}

// find first matching protocol in database; returns NULL if not found
static void* efi_find_protocol(EFI_HANDLE handle, EFI_GUID* guid) {
    serial_puts("[EFI] FindProtocol \n");
    if (!guid) return NULL;
    for (int i = 0; i < gProtocolCount; i++) {
        if (gProtocolDB[i].handle == handle &&
            efi_guid_match(&gProtocolDB[i].guid, guid))
            return gProtocolDB[i].iface;
    }
    return NULL;
}

static EFI_STATUS EFIAPI efi_LocateProtocol(
    EFI_GUID *guid, VOID *reg, VOID **iface
) {
    serial_puts("[EFI] LocateProtocol {");
    serial_putx(guid->Data1); serial_puts("-");
    serial_putx(guid->Data2); serial_puts("-");
    serial_putx(guid->Data3); serial_puts("}\n");

    // return the real LoadedImageProtocol if requested
    if (gLoadedImageInstance && efi_guid_match(guid, &gEfiLoadedImageProtocolGuid2)) {
        *iface = gLoadedImageInstance;
        serial_puts("  -> real LoadedImageProtocol\n");
        return EFI_SUCCESS;
    }

    // check protocol database for ANY handle
    if (gProtocolCount > 0) {
        for (int i = 0; i < gProtocolCount; i++) {
            if (efi_guid_match(&gProtocolDB[i].guid, guid)) {
                *iface = gProtocolDB[i].iface;
                return EFI_SUCCESS;
            }
        }
    }

    // allocate a minimal protocol interface: 16 stub function pointers
    uint64_t* proto = malloc(16 * sizeof(uint64_t));
    if (!proto) {
        *iface = NULL;
        return EFI_OUT_OF_RESOURCES;
    }
    for (int i = 0; i < 16; i++)
        proto[i] = (uint64_t)stub_Null;

    *iface = proto;
    return EFI_SUCCESS;
}

static EFI_STATUS efi_GetVariable(
    CHAR16   *VariableName,
    EFI_GUID *VendorGuid,
    UINT32   *Attributes,
    UINTN    *DataSize,
    VOID     *Data
) {
    serial_puts("[EFI] GetVarible name='");
    for (int i = 0; i < 10; i++) {
        serial_puts("uh");
        serial_putc(VariableName[i]);
    }
    serial_puts("'\n");
}

static EFI_STATUS EFIAPI stub_FreePool(void* a, void* b, void* c, void* d) {
    serial_puts("[EFI] FreePool\n");
    if (!a) return EFI_INVALID_PARAMETER;
    free(a);
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_GetMemoryMap(uint64_t *MemoryMapSize, 
    EFI_MEMORY_DESCRIPTOR *MemoryMap,
    uint64_t *MapKey, 
    uint64_t *DescriptorSize, 
    UINT32 *DescriptorVersion) 
{
    serial_puts("[EFI] MemoryMap path=");
    uint64_t required_size = memmap_length * sizeof(EFI_MEMORY_DESCRIPTOR);
    if (MemoryMap == NULL || *MemoryMapSize < required_size){
        *MemoryMapSize = required_size;
        *DescriptorSize = sizeof(EFI_MEMORY_DESCRIPTOR);
        serial_puts("(BUFFER TOO SMALL) descriptor_size=0x");
        serial_putx(sizeof(EFI_MEMORY_DESCRIPTOR));
        serial_puts(" requiered_size=0x");
        serial_putx(required_size);
        serial_puts("\n");
        return EFI_BUFFER_TOO_SMALL;
    }

    memmap_to_uefi(MemoryMap, required_size);

    serial_puts("(SUCCESS) MemoryMapSize=0x");
    serial_putx(*MemoryMapSize);
    serial_puts(" MapKey=0x");
    serial_putx(map_key);
    serial_puts(" DescriptorSize=0x");
    serial_putx(*DescriptorSize);
    serial_puts(" DescriptorVersion=");
    serial_putx(*DescriptorVersion);
    serial_puts("\n");

    *MemoryMapSize = memmap_length * sizeof(EFI_MEMORY_DESCRIPTOR);
    *MapKey = map_key;
    *DescriptorSize = sizeof(EFI_MEMORY_DESCRIPTOR);
    *DescriptorVersion = 1;
    return EFI_SUCCESS;
}

STUB(RaiseTPL,                      EFI_SUCCESS)
STUB(RestoreTPL,                    EFI_SUCCESS)
STUB(FreePages,                     EFI_SUCCESS)
STUB(CreateEvent,                   EFI_SUCCESS)
STUB(SetTimer,                      EFI_SUCCESS)
STUB(WaitForEvent,                  EFI_SUCCESS)
STUB(SignalEvent,                   EFI_SUCCESS)
STUB(CloseEvent,                    EFI_SUCCESS)
STUB(CheckEvent,                    EFI_NOT_READY)
STUB(ReinstallProtocolInterface,    EFI_SUCCESS)
STUB(UninstallProtocolInterface,    EFI_SUCCESS)
STUB(RegisterProtocolNotify,        EFI_SUCCESS)
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
STUB(CloseProtocol,                 EFI_SUCCESS)
STUB(OpenProtocolInformation,       EFI_SUCCESS)
STUB(ProtocolsPerHandle,            EFI_SUCCESS)
STUB(InstallMultipleProtocolInterfaces,   EFI_SUCCESS)
STUB(UninstallMultipleProtocolInterfaces, EFI_SUCCESS)

static EFI_STATUS EFIAPI efi_CalculateCrc32(VOID *Data, UINTN DataSize, UINT32 *CrcOut) {
    if (CrcOut) *CrcOut = crc32((uint8_t*)Data, DataSize);
    return EFI_SUCCESS;
}

// ── fake handle for protocol queries ───────────────────────────────

static int gFakeHandleData;
static EFI_HANDLE gFakeHandle = (EFI_HANDLE)&gFakeHandleData;

static EFI_STATUS EFIAPI efi_HandleProtocol(
    EFI_HANDLE handle,
    EFI_GUID*  protocol,
    VOID**     interface
) {
    serial_puts("[STUB] HandleProtocol\n");
    if (handle != NULL) {
        // check protocol database first
        void* found = efi_find_protocol(handle, protocol);
        if (found) {
            *interface = found;
            return EFI_SUCCESS;
        }
        // special: LoadedImageProtocol
        if (gLoadedImageInstance && efi_guid_match(protocol, &gEfiLoadedImageProtocolGuid2)) {
            *interface = gLoadedImageInstance;
            serial_puts("  -> real LoadedImageProtocol\n");
            return EFI_SUCCESS;
        }
        uint64_t* proto = malloc(16 * sizeof(uint64_t));
        if (!proto) {
            *interface = NULL;
            return EFI_OUT_OF_RESOURCES;
        }
        for (int i = 0; i < 16; i++)
            proto[i] = (uint64_t)stub_Null;
        *interface = proto;
        return EFI_SUCCESS;
    }
    *interface = NULL;
    return EFI_NOT_FOUND;
}

static EFI_STATUS EFIAPI efi_OpenProtocol(
    EFI_HANDLE handle,
    EFI_GUID*  protocol,
    VOID**     interface,
    EFI_HANDLE agent,
    EFI_HANDLE controller,
    UINT32     attributes
) {
    serial_puts("[STUB] OpenProtocol handle=");
    serial_putx((uint64_t)handle);
    serial_puts(" guid=");
    serial_putx(protocol->Data1); serial_putc('-');
    serial_putx(protocol->Data2); serial_putc('-');
    serial_putx(protocol->Data3);
    serial_puts("\n");
    if (handle != NULL) {
        // check protocol database first
        void* found = efi_find_protocol(handle, protocol);
        if (found) {
            *interface = found;
            return EFI_SUCCESS;
        }
        if (gLoadedImageInstance && efi_guid_match(protocol, &gEfiLoadedImageProtocolGuid2)) {
            *interface = gLoadedImageInstance;
            serial_puts("  -> real LoadedImageProtocol\n");
            return EFI_SUCCESS;
        }
        uint64_t* proto = malloc(16 * sizeof(uint64_t));
        if (!proto) {
            *interface = NULL;
            return EFI_OUT_OF_RESOURCES;
        }
        for (int i = 0; i < 16; i++)
            proto[i] = (uint64_t)stub_Null;
        *interface = proto;
        return EFI_SUCCESS;
    }
    *interface = NULL;
    return EFI_NOT_FOUND;
}

static EFI_STATUS EFIAPI efi_LocateHandleBuffer(
    EFI_LOCATE_SEARCH_TYPE type,
    EFI_GUID *guid,
    VOID *key,
    UINTN *count,
    EFI_HANDLE **buf
) {
    serial_puts("[STUB] LocateHandleBuffer\n");
    if (!count || !buf) return EFI_INVALID_PARAMETER;
    *count = 1;
    *buf = malloc(sizeof(EFI_HANDLE));
    if (!*buf) return EFI_OUT_OF_RESOURCES;
    (*buf)[0] = gFakeHandle;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_LocateHandle(
    EFI_LOCATE_SEARCH_TYPE type,
    EFI_GUID *guid,
    VOID *key,
    UINTN *count,
    EFI_HANDLE *buf
) {
    serial_puts("[STUB] LocateHandle\n");
    if (count) *count = 1;
    if (buf)  buf[0] = gFakeHandle;
    return EFI_SUCCESS;
}

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
    .GetMemoryMap                       = (void*)efi_GetMemoryMap,
    .AllocatePool                       = (void*)efi_AllocatePool,
    .FreePool                           = (void*)stub_FreePool,
    .CreateEvent                        = (void*)stub_CreateEvent,
    .SetTimer                           = (void*)stub_SetTimer,
    .WaitForEvent                       = (void*)stub_WaitForEvent,
    .SignalEvent                        = (void*)stub_SignalEvent,
    .CloseEvent                         = (void*)stub_CloseEvent,
    .CheckEvent                         = (void*)stub_CheckEvent,
    .InstallProtocolInterface           = (void*)efi_InstallProtocolInterface,
    .ReinstallProtocolInterface         = (void*)stub_ReinstallProtocolInterface,
    .UninstallProtocolInterface         = (void*)stub_UninstallProtocolInterface,
    .HandleProtocol                     = (void*)efi_HandleProtocol,
    .Reserved                           = NULL,
    .RegisterProtocolNotify             = (void*)stub_RegisterProtocolNotify,
    .LocateHandle                       = (void*)efi_LocateHandle,
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
    .OpenProtocol                       = (void*)efi_OpenProtocol,
    .CloseProtocol                      = (void*)stub_CloseProtocol,
    .OpenProtocolInformation            = (void*)stub_OpenProtocolInformation,
    .ProtocolsPerHandle                 = (void*)stub_ProtocolsPerHandle,
    .LocateHandleBuffer                 = (void*)efi_LocateHandleBuffer,
    .LocateProtocol                     = (void*)efi_LocateProtocol,
    .InstallMultipleProtocolInterfaces  = (void*)stub_InstallMultipleProtocolInterfaces,
    .UninstallMultipleProtocolInterfaces= (void*)stub_UninstallMultipleProtocolInterfaces,
    .CalculateCrc32                     = (void*)efi_CalculateCrc32,
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
    .GetVariable                = (void*)efi_GetVariable,
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