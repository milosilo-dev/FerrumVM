#pragma once

#include <stdint.h>
#include <stddef.h>

/* ============================================================
   Basic Types
   ============================================================ */

typedef uint8_t     BOOLEAN;

typedef int8_t      INT8;
typedef int16_t     INT16;
typedef int32_t     INT32;
typedef int64_t     INT64;

typedef uint8_t     UINT8;
typedef uint16_t    UINT16;
typedef uint32_t    UINT32;
typedef uint64_t    UINT64;

typedef INT64       INTN;
typedef UINT64      UINTN;

typedef UINT64      EFI_STATUS;

typedef void*       EFI_HANDLE;
typedef void        VOID;

typedef UINT16      CHAR16;

#define EFI_SUCCESS 0

#define EFIAPI __attribute__((ms_abi))

/* ============================================================
   Table Header
   ============================================================ */

typedef struct {
    UINT64 Signature;
    UINT32 Revision;
    UINT32 HeaderSize;
    UINT32 CRC32;
    UINT32 Reserved;
} EFI_TABLE_HEADER;

/* ============================================================
   Memory Types
   ============================================================ */

typedef enum {
    EfiReservedMemoryType,
    EfiLoaderCode,
    EfiLoaderData,
    EfiBootServicesCode,
    EfiBootServicesData,
    EfiRuntimeServicesCode,
    EfiRuntimeServicesData,
    EfiConventionalMemory,
    EfiUnusableMemory,
    EfiACPIReclaimMemory,
    EfiACPIMemoryNVS,
    EfiMemoryMappedIO,
    EfiMemoryMappedIOPortSpace,
    EfiPalCode,
    EfiPersistentMemory,
    EfiMaxMemoryType
} EFI_MEMORY_TYPE;

/* ============================================================
   Console Output Protocol
   ============================================================ */

typedef struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL;

typedef struct SIMPLE_TEXT_OUTPUT_MODE
    SIMPLE_TEXT_OUTPUT_MODE;

typedef EFI_STATUS (EFIAPI *EFI_TEXT_RESET)(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* This,
    BOOLEAN ExtendedVerification
);

typedef EFI_STATUS (EFIAPI *EFI_TEXT_STRING)(
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* This,
    CHAR16* String
);

struct EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    EFI_TEXT_RESET Reset;
    EFI_TEXT_STRING OutputString;

    VOID* TestString;
    VOID* QueryMode;
    VOID* SetMode;
    VOID* SetAttribute;
    VOID* ClearScreen;
    VOID* SetCursorPosition;
    VOID* EnableCursor;

    SIMPLE_TEXT_OUTPUT_MODE* Mode;
};

/* ============================================================
   Boot Services Function Typedefs
   ============================================================ */

typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_PAGES)(
    UINTN Type,
    EFI_MEMORY_TYPE MemoryType,
    UINTN Pages,
    UINT64* Memory
);

typedef EFI_STATUS (EFIAPI *EFI_FREE_PAGES)(
    UINT64 Memory,
    UINTN Pages
);

typedef EFI_STATUS (EFIAPI *EFI_GET_MEMORY_MAP)(
    UINTN* MemoryMapSize,
    VOID* MemoryMap,
    UINTN* MapKey,
    UINTN* DescriptorSize,
    UINT32* DescriptorVersion
);

typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_POOL)(
    EFI_MEMORY_TYPE PoolType,
    UINTN Size,
    VOID** Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_FREE_POOL)(
    VOID* Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_STALL)(
    UINTN Microseconds
);

typedef EFI_STATUS (EFIAPI *EFI_CALCULATE_CRC32)(
    VOID* Data,
    UINTN DataSize,
    UINT32* Crc32
);

typedef EFI_STATUS (EFIAPI *EFI_CREATE_EVENT)(
    UINT32 Type,
    UINTN NotifyTpl,
    VOID* NotifyFunction,
    VOID* NotifyContext,
    VOID** Event
);

typedef EFI_STATUS (EFIAPI *EFI_CLOSE_EVENT)(
    VOID* Event
);

typedef EFI_STATUS (EFIAPI *EFI_WAIT_FOR_EVENT)(
    UINTN NumberOfEvents,
    VOID** Event,
    UINTN* Index
);

typedef EFI_STATUS (EFIAPI *EFI_SIGNAL_EVENT)(
    VOID* Event
);

typedef EFI_STATUS (EFIAPI *EFI_CHECK_EVENT)(
    VOID* Event
);

typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE_BUFFER)(
    UINTN SearchType,
    VOID* Protocol,
    VOID* SearchKey,
    UINTN* NoHandles,
    EFI_HANDLE** Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_EXIT_BOOT_SERVICES)(
    EFI_HANDLE ImageHandle,
    UINTN MapKey
);

/* ============================================================
   Boot Services
   ============================================================ */

typedef struct EFI_BOOT_SERVICES {
    EFI_TABLE_HEADER Header;

    VOID* RaiseTPL;
    VOID* RestoreTPL;

    EFI_ALLOCATE_PAGES AllocatePages;
    EFI_FREE_PAGES FreePages;
    EFI_GET_MEMORY_MAP GetMemoryMap;

    EFI_ALLOCATE_POOL AllocatePool;
    EFI_FREE_POOL FreePool;

    EFI_CREATE_EVENT CreateEvent;
    VOID* SetTimer;
    EFI_WAIT_FOR_EVENT WaitForEvent;
    EFI_SIGNAL_EVENT SignalEvent;
    EFI_CLOSE_EVENT CloseEvent;
    EFI_CHECK_EVENT CheckEvent;

    VOID* InstallProtocolInterface;
    VOID* ReinstallProtocolInterface;
    VOID* UninstallProtocolInterface;
    VOID* HandleProtocol;

    VOID* Reserved;

    VOID* RegisterProtocolNotify;
    VOID* LocateHandle;
    VOID* LocateDevicePath;
    VOID* InstallConfigurationTable;

    VOID* LoadImage;
    VOID* StartImage;
    VOID* Exit;
    VOID* UnloadImage;

    EFI_EXIT_BOOT_SERVICES ExitBootServices;

    VOID* GetNextMonotonicCount;

    EFI_STALL Stall;

    VOID* SetWatchdogTimer;

    VOID* ConnectController;
    VOID* DisconnectController;

    VOID* OpenProtocol;
    VOID* CloseProtocol;
    VOID* OpenProtocolInformation;

    VOID* ProtocolsPerHandle;

    EFI_LOCATE_HANDLE_BUFFER LocateHandleBuffer;

    VOID* LocateProtocol;
    VOID* InstallMultipleProtocolInterfaces;
    VOID* UninstallMultipleProtocolInterfaces;

    EFI_CALCULATE_CRC32 CalculateCrc32;

    VOID* CopyMem;
    VOID* SetMem;

    VOID* CreateEventEx;

} EFI_BOOT_SERVICES;

/* ============================================================
   Runtime Services Placeholder
   ============================================================ */

typedef struct {
    EFI_TABLE_HEADER Header;
} EFI_RUNTIME_SERVICES;

/* ============================================================
   Configuration Table
   ============================================================ */

typedef struct {
    UINT8 VendorGuid[16];
    VOID* VendorTable;
} EFI_CONFIGURATION_TABLE;

/* ============================================================
   System Table
   ============================================================ */

typedef struct {
    EFI_TABLE_HEADER Header;

    CHAR16* FirmwareVendor;
    UINT32 FirmwareRevision;

    EFI_HANDLE ConsoleInHandle;
    VOID* ConIn;

    EFI_HANDLE ConsoleOutHandle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* ConOut;

    EFI_HANDLE StandardErrorHandle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* StdErr;

    EFI_RUNTIME_SERVICES* RuntimeServices;
    EFI_BOOT_SERVICES* BootServices;

    UINTN NumberOfTableEntries;
    EFI_CONFIGURATION_TABLE* ConfigurationTable;

} EFI_SYSTEM_TABLE;

typedef struct {
    UINT32 Data1;
    UINT16 Data2;
    UINT16 Data3;
    UINT8  Data4[8];
} EFI_GUID;