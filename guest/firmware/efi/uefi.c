#include "uefi.h"
#include "../headers/serial.h"
#include "../headers/uefi/crc32.h"
#include "../headers/uefi/stip.h"
#include "../mem/heap.h"

#define TSC_MHZ 3000

#define STUB(name, ret) \
    static EFI_STATUS EFIAPI stub_##name( \
        void* a, void* b, void* c, void* d) { \
        serial2_puts("[STUB] " #name "\n"); \
        return ret; \
    }

static EFI_STATUS EFIAPI stub_Null() {
    return EFI_SUCCESS;
}

static uint64_t map_key = 1;
static int gExitedBootServices = 0;
static uint64_t gMonotonicCount = 0;
static int move_count = 0;

void update_events() {
    if (serial_isdata()) {
        gConIn.WaitForKey->signaled = true;
    }
}

static EFI_STATUS EFIAPI efi_output_string(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *This,
    uint16_t *String
) {
    move_count = 0;  // letter was printed, reset the streak
    while (*String) {
        char c = (char)(*String & 0xFF);
        if (c == '\n') serial_putc('\n');
        else serial_putc(c);
        String++;
    }
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_Stall(UINTN microseconds) {
    serial2_puts("[EFI] Stall us=");
    serial2_putx(microseconds);
    serial2_puts("\n");

    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_SetCursorPosition() {
    move_count++;
    if (move_count == 2) {
        serial_puts("\n");
    }
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI stub_EnableCursor() {
    return EFI_SUCCESS;
}

STUB(ConReset,        EFI_SUCCESS)
STUB(TestString,      EFI_SUCCESS)
STUB(QueryMode,       EFI_SUCCESS)
STUB(SetMode,         EFI_SUCCESS)
STUB(SetAttribute,    EFI_SUCCESS)
STUB(ClearScreen,     EFI_SUCCESS)

static SIMPLE_TEXT_OUTPUT_MODE gConOutMode = {
    .MaxMode       = 1,
    .Mode          = 0,
    .Attribute     = 0,
    .CursorColumn  = 0,
    .CursorRow     = 0,
    .CursorVisible = 1,
};

static EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL gConOut = {
    .Reset          = (void*)stub_ConReset,
    .OutputString   = efi_output_string,
    .TestString     = (void*)stub_TestString,
    .QueryMode      = (void*)stub_QueryMode,
    .SetMode        = (void*)stub_SetMode,
    .SetAttribute   = (void*)stub_SetAttribute,
    .ClearScreen    = (void*)stub_ClearScreen,
    .SetCursorPosition  = (void*)stub_SetCursorPosition,
    .EnableCursor       = (void*)stub_EnableCursor,
    .Mode               = &gConOutMode,
};

#define PG_LIMIT 0x20000000ULL
static uint64_t pg_bump = 0x4000000ULL;

static EFI_STATUS EFIAPI efi_AllocatePool(
    EFI_MEMORY_TYPE type,
    UINTN size,
    VOID **out
) {
    serial2_puts("[EFI] AlloactePool\n");
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
    (void)memory_type;

    serial2_puts("[EFI] AllocatePages type=0x");
    serial2_putx(type);
    serial2_puts(" pages=0x");
    serial2_putx(pages);

    if (pages == 0 || memory == NULL) {
        serial2_puts(" ret=EFI_INVALID_PARAMETER\n");
        return EFI_INVALID_PARAMETER;
    }

    uint64_t size = pages * 4096ULL;
    uint64_t addr = 0;

    switch (type) {

    case AllocateAddress: {
        addr = *memory;

        serial2_puts(" requested=0x");
        serial2_putx(addr);

        if (addr & 0xFFFULL) {
            serial2_puts(" ret=EFI_INVALID_PARAMETER\n");
            return EFI_INVALID_PARAMETER;
        }

        if (addr < 0x200000ULL) {
            serial2_puts(" ret=EFI_OUT_OF_RESOURCES\n");
            return EFI_OUT_OF_RESOURCES;
        }

        if (addr + size > PG_LIMIT) {
            serial2_puts(" ret=EFI_OUT_OF_RESOURCES\n");
            return EFI_OUT_OF_RESOURCES;
        }
        *memory = addr;
        map_key++;
        serial2_puts(" ret=EFI_SUCCESS\n");
        return EFI_SUCCESS;
    }

    case AllocateMaxAddress: {
        uint64_t max = *memory;

        if (max < size) {
            serial2_puts(" ret=EFI_OUT_OF_RESOURCES\n");
            return EFI_OUT_OF_RESOURCES;
        }

        addr = (max - size) & ~0xFFFULL;

        if (addr < 0x200000ULL) {
            addr = 0x200000ULL;
        }

        if (addr + size > PG_LIMIT) {
            serial2_puts(" ret=EFI_OUT_OF_RESOURCES\n");
            return EFI_OUT_OF_RESOURCES;
        }

        serial2_puts(" returned=0x");
        serial2_putx(addr);

        break;
    }

    case AllocateAnyPages:
    default: {
        addr = (pg_bump + 0xFFFULL) & ~0xFFFULL;

        serial2_puts(" returned=0x");
        serial2_putx(addr);

        if (addr + size > PG_LIMIT) {
            serial2_puts(" ret=EFI_OUT_OF_RESOURCES\n");
            return EFI_OUT_OF_RESOURCES;
        }

        pg_bump = addr + size;

        break;
    }
    }

    *memory = addr;
    map_key++;
    memset((void *)(uintptr_t)addr, 0, size);
    serial2_puts(" ret=EFI_SUCCESS\n");

    return EFI_SUCCESS;
}

static EFI_LOADED_IMAGE_PROTOCOL gLoadedImageData = {
    .Revision     = EFI_LOADED_IMAGE_PROTOCOL_REVISION,
    .ImageBase    = NULL,
    .ImageSize    = 0,
};
EFI_LOADED_IMAGE_PROTOCOL *gLoadedImageInstance = &gLoadedImageData;

static int efi_guid_match(EFI_GUID* a, EFI_GUID* b) {
    return memcmp(a, b, sizeof(EFI_GUID)) == 0;
}

static EFI_GUID gEfiLoadedImageProtocolGuid2 = {
    0x5B1B31A1, 0x9562, 0x11D2,
    {0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

#define MAX_PROTOCOLS 64
static struct {
    EFI_HANDLE handle;
    EFI_GUID   guid;
    void*      iface;
} gProtocolDB[MAX_PROTOCOLS];
static int gProtocolCount;

static void efi_dump_protocols_for(EFI_HANDLE handle) {
    serial2_puts("[EFI] Protocols on handle ");
    serial2_putx((uint64_t)handle);
    serial2_puts(":\n");
    for (int i = 0; i < gProtocolCount; i++) {
        if (gProtocolDB[i].handle == handle) {
            serial2_puts("  GUID=");
            serial2_putx(gProtocolDB[i].guid.Data1); serial2_puts("-");
            serial2_putx(gProtocolDB[i].guid.Data2); serial2_puts("-");
            serial2_putx(gProtocolDB[i].guid.Data3); serial2_puts("-");
            for (int j = 0; j < 8; j++)
                serial2_putx(gProtocolDB[i].guid.Data4[j]);
            serial2_puts(" iface=");
            serial2_putx((uint64_t)gProtocolDB[i].iface);
            serial2_puts("\n");
        }
    }
}

static EFI_STATUS EFIAPI efi_InstallProtocolInterface(
    EFI_HANDLE *Handle,
    EFI_GUID   *Protocol,
    VOID       *Interface,
    VOID       *DevicePath
) {
    serial2_puts("[EFI] InstallProtocolInterface\n");
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

static void* efi_find_protocol(EFI_HANDLE handle, EFI_GUID* guid) {
    if (!guid) return NULL;
    for (int i = 0; i < gProtocolCount; i++) {
        if (gProtocolDB[i].handle == handle &&
            efi_guid_match(&gProtocolDB[i].guid, guid))
            return gProtocolDB[i].iface;
    }
    return NULL;
}

void efi_register_protocol(EFI_HANDLE handle, EFI_GUID *guid, void *iface) {
    serial2_puts("[FIRMWARE] Installed protcol with guid {");
    serial2_putx(guid->Data1); serial2_puts("-");
    serial2_putx(guid->Data2); serial2_puts("-");
    serial2_putx(guid->Data3); serial2_puts("} onto handle 0x");
    serial2_putx((uint32_t)handle); serial2_puts(" and iface 0x");
    serial2_putx((uint32_t)iface); serial2_puts("\n");

    if (gProtocolCount >= MAX_PROTOCOLS) {
        serial2_puts("[EFI] Protocol DB full!\n");
        return;
    }
    gProtocolDB[gProtocolCount].handle = handle;
    gProtocolDB[gProtocolCount].guid   = *guid;
    gProtocolDB[gProtocolCount].iface  = iface;
    gProtocolCount++;
}

static EFI_STATUS EFIAPI efi_LocateProtocol(
    EFI_GUID *guid, VOID *reg, VOID **iface
) {
    serial2_puts("[EFI] LocateProtocol {");
    serial2_putx(guid->Data1); serial2_puts("-");
    serial2_putx(guid->Data2); serial2_puts("-");
    serial2_putx(guid->Data3); serial2_puts("} ret=");

    if (gLoadedImageInstance && efi_guid_match(guid, &gEfiLoadedImageProtocolGuid2)) {
        *iface = gLoadedImageInstance;
        serial2_puts("LoadedImageProtocol\n");
        return EFI_SUCCESS;
    }

    if (gProtocolCount > 0) {
        for (int i = 0; i < gProtocolCount; i++) {
            if (efi_guid_match(&gProtocolDB[i].guid, guid)) {
                *iface = gProtocolDB[i].iface;
                serial2_puts("efi_sucsess\n");
                return EFI_SUCCESS;
            }
        }
    }

    uint64_t* proto = malloc(16 * sizeof(uint64_t));
    if (!proto) {
        *iface = NULL;
        serial2_puts("out_of_resources\n");
        return EFI_OUT_OF_RESOURCES;
    }
    for (int i = 0; i < 16; i++)
        proto[i] = (uint64_t)stub_Null;

    *iface = proto;
    serial2_puts("efi_sucsess\n");
    return EFI_SUCCESS;
}

static EFI_STATUS efi_GetVariable(
    CHAR16   *VariableName,
    EFI_GUID *VendorGuid,
    UINT32   *Attributes,
    UINTN    *DataSize,
    VOID     *Data
) {
    serial2_puts("[EFI] GetVarible\n");
}

static EFI_STATUS EFIAPI stub_FreePool(void* a, void* b, void* c, void* d) {
    serial2_puts("[EFI] FreePool\n");
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
    serial2_puts("[EFI] MemoryMap path=");
    uint64_t required_size = memmap_length * sizeof(EFI_MEMORY_DESCRIPTOR);
    if (MemoryMap == NULL || *MemoryMapSize < required_size){
        *MemoryMapSize = required_size;
        *DescriptorSize = sizeof(EFI_MEMORY_DESCRIPTOR);
        serial2_puts("(BUFFER TOO SMALL) descriptor_size=0x");
        serial2_putx(sizeof(EFI_MEMORY_DESCRIPTOR));
        serial2_puts(" requiered_size=0x");
        serial2_putx(required_size);
        serial2_puts("\n");
        return EFI_BUFFER_TOO_SMALL;
    }

    memmap_to_uefi(MemoryMap, required_size);

    serial2_puts("(SUCCESS) MemoryMapSize=0x");
    serial2_putx(*MemoryMapSize);
    serial2_puts(" MapKey=0x");
    serial2_putx(map_key);
    serial2_puts(" DescriptorSize=0x");
    serial2_putx(*DescriptorSize);
    serial2_puts(" DescriptorVersion=");
    serial2_putx(*DescriptorVersion);
    serial2_puts("\n");

    *MemoryMapSize = memmap_length * sizeof(EFI_MEMORY_DESCRIPTOR);
    *MapKey = map_key;
    *DescriptorSize = sizeof(EFI_MEMORY_DESCRIPTOR);
    *DescriptorVersion = 1;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_WaitForEvent(UINTN NumberOfEvents,
    EFI_EVENT  *Events,
    UINTN     *Index) 
{
    serial2_puts("[EFI] WaitForEvent type=0x");
    serial2_putx(Events[0]->type);
    serial2_puts(" length=0x");
    serial2_putx(NumberOfEvents);
    serial2_puts("\n");

    while (1) {
        for (UINTN i = 0; i < NumberOfEvents; i++) {
            INTERNAL_EVENT* Event = Events[i];

            if (Event->signaled) {
                Event->signaled = false;
                *Index = i;
                return EFI_SUCCESS;
            }
        }
        update_events();
    }
}

STUB(RaiseTPL,                      EFI_SUCCESS)
STUB(RestoreTPL,                    EFI_SUCCESS)
STUB(FreePages,                     EFI_SUCCESS)
static EFI_STATUS EFIAPI efi_CreateEvent(
    UINT32 Type, EFI_TPL NotifyTpl,
    void *NotifyFunction, void *NotifyContext,
    EFI_EVENT *Event
) {
    serial2_puts("[EFI] CreateEvent type=0x");
    serial2_putx(Type);
    serial2_puts(" tpl=0x");
    serial2_putx(NotifyTpl);
    serial2_puts("\n");

    if (!Event) return EFI_INVALID_PARAMETER;

    INTERNAL_EVENT *ev = malloc(sizeof(INTERNAL_EVENT));
    if (!ev) return EFI_OUT_OF_RESOURCES;

    ev->signaled = 0;
    ev->notify   = NotifyFunction;
    ev->context  = NotifyContext;
    ev->type     = (uint8_t)Type;
    *Event = ev;

    serial2_puts("[EFI] CreateEvent -> event=0x");
    serial2_putx((uint64_t)ev);
    serial2_puts("\n");
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_CreateEventEx(
    UINT32 Type, EFI_TPL NotifyTpl,
    void *NotifyFunction, void *NotifyContext,
    EFI_GUID *EventGroup, EFI_EVENT *Event
) {
    serial2_puts("[EFI] CreateEventEx type=0x");
    serial2_putx(Type);
    serial2_puts("\n");
    (void)EventGroup;

    if (!Event) return EFI_INVALID_PARAMETER;
    INTERNAL_EVENT *ev = malloc(sizeof(INTERNAL_EVENT));
    if (!ev) return EFI_OUT_OF_RESOURCES;

    ev->signaled = 0;
    ev->notify   = NotifyFunction;
    ev->context  = NotifyContext;
    ev->type     = (uint8_t)Type;
    *Event = ev;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_SetTimer(EFI_EVENT Event, EFI_TIMER_DELAY Type, UINT64 TriggerTime) {
    Event->signaled = true;
    return EFI_SUCCESS;
}

STUB(SetTimer,                      EFI_SUCCESS)
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

static EFI_STATUS EFIAPI efi_ExitBootServices(
    EFI_HANDLE ImageHandle, UINTN MapKey
) {
    serial2_puts("[EFI] ExitBootServices key=0x");
    serial2_putx(MapKey);
    serial2_puts(" current_key=0x");
    serial2_putx(map_key);
    serial2_puts("\n");

    (void)ImageHandle;

    if (MapKey != map_key) {
        serial2_puts("[EFI] ExitBootServices: invalid map key\n");
        return EFI_INVALID_PARAMETER;
    }

    gExitedBootServices = 1;
    serial2_puts("[EFI] ExitBootServices: SUCCESS — boot services disabled\n");
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_GetNextMonotonicCount(
    UINT64 *Count
) {
    if (!Count) return EFI_INVALID_PARAMETER;
    *Count = gMonotonicCount++;
    serial2_puts("[EFI] GetNextMonotonicCount count=0x");
    serial2_putx(*Count);
    serial2_puts("\n");
    return EFI_SUCCESS;
}
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

static int gFakeHandleData;
static EFI_HANDLE gFakeHandle = (EFI_HANDLE)&gFakeHandleData;

static EFI_STATUS EFIAPI efi_HandleProtocol(
    EFI_HANDLE handle, EFI_GUID* protocol, VOID** interface
) {
    serial2_puts("[EFI] HandleProtocol ");

    if (!handle || !interface) {
        serial2_puts("ret=invalid_param, handle=");
        serial2_putx((uint64_t)handle);
        serial2_puts(", interface=");
        serial2_putx((uint64_t)interface);
        serial2_puts("\n");
        return EFI_INVALID_PARAMETER;
    }

    serial2_puts("handle=");
    serial2_putx((uint64_t)handle);
    serial2_puts(" ret=");
    
    void* found = efi_find_protocol(handle, protocol);
    if (found) {
        *interface = found;
        serial2_puts("efi_sucsess\n");
        return EFI_SUCCESS;
    }

    if (gLoadedImageInstance && efi_guid_match(protocol, &gEfiLoadedImageProtocolGuid2)) {
        serial2_puts("LoadedImage DeviceHandle=");
        serial2_putx((uint64_t)gLoadedImageInstance->DeviceHandle);
        serial2_puts("\n");

        *interface = gLoadedImageInstance;
        return EFI_SUCCESS;
    }

    *interface = NULL;
    serial2_puts("un-suported\n");
    return EFI_UNSUPPORTED;
}

static EFI_STATUS EFIAPI efi_OpenProtocol(
    EFI_HANDLE handle, EFI_GUID* protocol, VOID** interface,
    EFI_HANDLE agent, EFI_HANDLE controller, UINT32 attributes
) {
    serial2_puts("[EFI] OpenProtocol\n");
    return efi_HandleProtocol(handle, protocol, interface);
}

static EFI_STATUS EFIAPI efi_LocateHandleBuffer(
    EFI_LOCATE_SEARCH_TYPE type,
    EFI_GUID *guid,
    VOID *key,
    UINTN *count,
    EFI_HANDLE **buf
) {
    serial2_puts("[EFI] LocateHandleBuffer {");
    serial2_putx(guid->Data1);serial2_puts("-");
    serial2_putx(guid->Data2);serial2_puts("-");
    serial2_putx(guid->Data3);serial2_puts("}\n");
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
    UINTN *BufferSize,
    EFI_HANDLE *Buffer
) {
    serial2_puts("[EFI] LocateHandle {");
    serial2_putx(guid->Data1);serial2_puts("-");
    serial2_putx(guid->Data2);serial2_puts("-");
    serial2_putx(guid->Data3);serial2_puts("} ret=");

    if (!BufferSize) {
        serial2_puts("invalid_parameter\n");
        return EFI_INVALID_PARAMETER;
    }

    EFI_HANDLE matches[MAX_PROTOCOLS];
    UINTN      nmatches = 0;

    for (UINTN i = 0; i < gProtocolCount; i++) {
        if (!efi_guid_match(&gProtocolDB[i].guid, guid)) continue;
        bool already = false;
        for (UINTN j = 0; j < nmatches; j++)
            if (matches[j] == gProtocolDB[i].handle) { already = true; break; }
        if (!already)
            matches[nmatches++] = gProtocolDB[i].handle;
    }

    if (nmatches == 0) {
        *BufferSize = 0;
        serial2_puts("not_found\n");
        return EFI_NOT_FOUND;
    }

    UINTN required = nmatches * sizeof(EFI_HANDLE);
    if (Buffer == NULL || *BufferSize < required) {
        *BufferSize = required;
        serial2_puts("buffer_too_small\n");
        return EFI_BUFFER_TOO_SMALL;
    }

    *BufferSize = required;
    for (UINTN i = 0; i < nmatches; i++)
        Buffer[i] = matches[i];
    serial2_puts("sucsess\n");
    return EFI_SUCCESS;
}

static VOID EFIAPI stub_CopyMem(VOID *dst, VOID *src, UINTN len) { memcpy(dst, src, len); }
static VOID EFIAPI stub_SetMem(VOID *buf, UINTN size, UINT8 val) { memset(buf, val, size); }

static EFI_STATUS EFIAPI stub_Exit(EFI_HANDLE img, EFI_STATUS status, UINTN size, CHAR16 *data) {
    serial2_puts("[STUB] Exit — halting\n");
    for (;;) __asm__("hlt");
}

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
    .CreateEvent                        = (void*)efi_CreateEvent,
    .SetTimer                           = (void*)efi_SetTimer,
    .WaitForEvent                       = (void*)efi_WaitForEvent,
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
    .ExitBootServices                   = (void*)efi_ExitBootServices,
    .GetNextMonotonicCount              = (void*)efi_GetNextMonotonicCount,
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
    .CreateEventEx                      = (void*)efi_CreateEventEx,
};

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

void format_config_table() {
    gConfigTables[0].VendorGuid = (EFI_GUID)ACPI_20_TABLE_GUID;
    gConfigTables[0].VendorTable = (void*)(0xE0000);

    gConfigTables[1].VendorGuid = (EFI_GUID)ACPI_TABLE_GUID;
    gConfigTables[1].VendorTable = (void*)(0xE0000);
}

static uint16_t gFirmwareVendor[] = { 'F','e','r','r','u','m', 0 };

void patch_null_stubs(void) {
    void** tbl = (void**)&gBootServices;
    for (int i = 5; i < sizeof(EFI_BOOT_SERVICES)/8; i++) {
        if (tbl[i] == NULL)
            tbl[i] = stub_Null;
    }

    tbl = (void**)&gRuntimeServices;
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
    handle_data->loaded_image.Revision    = EFI_LOADED_IMAGE_PROTOCOL_REVISION;
    handle_data->loaded_image.ImageBase   = load_base;
    handle_data->loaded_image.ImageSize   = image_size;
    handle_data->loaded_image.SystemTable = st;
    handle_data->loaded_image.FilePath    = (EFI_DEVICE_PATH_PROTOCOL*)&gBootFilePath;

    serial2_puts("Boot file path = ");
    serial2_putx(gBootFilePath.FileNode.Length[0]);
    serial2_puts("\n");
}

void efi_init(EFI_SYSTEM_TABLE *st, EFI_HANDLE image_handle) {
    efi_register_protocol(image_handle,
                          &gEfiLoadedImageProtocolGuid2,
                          gLoadedImageInstance);
    
    gDiskMedia.LastBlock = (virtio_blk_config.capacity / virtio_blk_config.blk_size) - 1;
    gDiskMedia.BlockSize = virtio_blk_config.blk_size;

    serial2_puts("Virtio Blk Last Block = 0x");
    serial2_putx(gDiskMedia.LastBlock);
    serial2_puts("\nVirtio Blk Size = 0x");
    serial2_putx(gDiskMedia.BlockSize);
    serial2_puts("\nVirtio Blk Bytes = 0x");
    serial2_putx(gDiskMedia.LastBlock * gDiskMedia.BlockSize);
    serial2_puts("\n");

    serial2_puts("[EFI] DevicePath iface ptr = 0x");
    serial2_putx((uint64_t)&gDiskPath);
    serial2_puts("\n");

    serial2_puts("[DEBUG] gDiskPath before register:\n");
    serial2_puts("  Type="); serial2_putx(gDiskPath.Hd.Header.Type);
    serial2_puts(" SubType="); serial2_putx(gDiskPath.Hd.Header.SubType);
    serial2_puts(" PartNum="); serial2_putx(gDiskPath.Hd.PartitionNumber);
    serial2_puts(" Start="); serial2_putx(gDiskPath.Hd.PartitionStart);
    serial2_puts(" Size="); serial2_putx(gDiskPath.Hd.PartitionSize);
    serial2_puts(" SigType="); serial2_putx(gDiskPath.Hd.SignatureType);
    serial2_puts("\n");

    efi_register_protocol(gDiskHandle,
                        &gEfiBlockIoProtocolGuid,
                        &gBlockIo);

    efi_register_protocol(gDiskHandle, 
                        &gEfiDevicePathProtocolGuid, 
                        &gDiskPath);
    
    efi_register_protocol(gDiskHandle, 
                        &gEfiSimpleFileSystemProtocolGuid, 
                        &gSfsp);

    gLoadedImageInstance->DeviceHandle = gDiskHandle;

    format_config_table();
    st->ConfigurationTable   = gConfigTables;
    st->NumberOfTableEntries = 2;
    st->Hdr.CRC32 = 0;
    st->Hdr.CRC32 = crc32((uint8_t*)st, st->Hdr.HeaderSize);
}