#pragma once
#include <stdint.h>

typedef uint64_t  UINTN;
typedef int64_t   INTN;
typedef uint64_t  EFI_STATUS;
typedef uint64_t  EFI_PHYSICAL_ADDRESS;
typedef uint64_t  EFI_VIRTUAL_ADDRESS;
typedef void*     EFI_HANDLE;
typedef void*     EFI_EVENT;
typedef uint64_t  EFI_TPL;
typedef uint32_t  UINT32;
typedef uint16_t  UINT16;
typedef uint8_t   UINT8;
typedef uint16_t  CHAR16;
typedef void      VOID;
typedef int       BOOLEAN;

#define EFIAPI __attribute__((ms_abi))
#define NULL (void*)0

// ── status codes ──────────────────────────────────────────────────
#define EFI_SUCCESS             0ULL
#define EFI_UNSUPPORTED         (0x8000000000000000ULL | 3)
#define EFI_NOT_FOUND           (0x8000000000000000ULL | 14)
#define EFI_OUT_OF_RESOURCES    (0x8000000000000000ULL | 9)
#define EFI_BUFFER_TOO_SMALL    (0x8000000000000000ULL | 5)
#define EFI_NOT_READY           (0x8000000000000000ULL | 6)

// ── GUID ──────────────────────────────────────────────────────────
typedef struct {
    uint32_t Data1;
    uint16_t Data2;
    uint16_t Data3;
    uint8_t  Data4[8];
} EFI_GUID;

// ── table header ──────────────────────────────────────────────────
typedef struct {
    uint64_t Signature;
    uint32_t Revision;
    uint32_t HeaderSize;
    uint32_t CRC32;
    uint32_t Reserved;
} EFI_TABLE_HEADER;

// ── memory ────────────────────────────────────────────────────────
typedef uint32_t EFI_MEMORY_TYPE;
typedef uint32_t EFI_ALLOCATE_TYPE;
typedef uint32_t EFI_LOCATE_SEARCH_TYPE;
typedef uint32_t EFI_INTERFACE_TYPE;
typedef uint32_t EFI_TIMER_DELAY;

#define AllocateAnyPages        0
#define AllocateMaxAddress      1
#define AllocateAddress         2

#define EfiConventionalMemory   7
#define EfiLoaderData           2

#define EFI_MEMORY_WB           0x8ULL
#define EFI_MEMORY_DESCRIPTOR_VERSION 1

typedef struct {
    uint32_t              Type;
    uint32_t              Pad;
    EFI_PHYSICAL_ADDRESS  PhysicalStart;
    EFI_VIRTUAL_ADDRESS   VirtualStart;
    uint64_t              NumberOfPages;
    uint64_t              Attribute;
} EFI_MEMORY_DESCRIPTOR;

// ── open protocol information ─────────────────────────────────────
typedef struct {
    EFI_HANDLE  AgentHandle;
    EFI_HANDLE  ControllerHandle;
    uint32_t    Attributes;
    uint32_t    OpenCount;
} EFI_OPEN_PROTOCOL_INFORMATION_ENTRY;

// ── device path (minimal) ─────────────────────────────────────────
typedef struct {
    uint8_t Type;
    uint8_t SubType;
    uint8_t Length[2];
} EFI_DEVICE_PATH_PROTOCOL;

// ── simple text output ────────────────────────────────────────────
typedef struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL;
typedef EFI_STATUS (EFIAPI *EFI_TEXT_RESET)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_STRING)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, CHAR16*);

typedef struct {
    UINT32   MaxMode;
    UINT32   Mode;
    UINT32   Attribute;
    UINT32   CursorColumn;
    UINT32   CursorRow;
    BOOLEAN CursorVisible;
} SIMPLE_TEXT_OUTPUT_MODE;

struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    EFI_TEXT_RESET   Reset;
    EFI_TEXT_STRING  OutputString;
    void *TestString, *QueryMode, *SetMode, *SetAttribute, *ClearScreen,
         *SetCursorPosition, *EnableCursor, *Mode;
};

// ── boot services function typedefs ───────────────────────────────
typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_PAGES)   (EFI_ALLOCATE_TYPE, EFI_MEMORY_TYPE, UINTN, EFI_PHYSICAL_ADDRESS*);
typedef EFI_STATUS (EFIAPI *EFI_FREE_PAGES)        (EFI_PHYSICAL_ADDRESS, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_GET_MEMORY_MAP)    (UINTN*, EFI_MEMORY_DESCRIPTOR*, UINTN*, UINTN*, UINT32*);
typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_POOL)     (EFI_MEMORY_TYPE, UINTN, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_FREE_POOL)         (VOID*);
typedef EFI_STATUS (EFIAPI *EFI_EXIT_BOOT_SERVICES)(EFI_HANDLE, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_SET_WATCHDOG_TIMER)(UINTN, uint64_t, UINTN, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_HANDLE_PROTOCOL)   (EFI_HANDLE, EFI_GUID*, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_PROTOCOL)   (EFI_GUID*, VOID*, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_OPEN_PROTOCOL)     (EFI_HANDLE, EFI_GUID*, VOID**, EFI_HANDLE, EFI_HANDLE, UINT32);
typedef EFI_STATUS (EFIAPI *EFI_CLOSE_PROTOCOL)    (EFI_HANDLE, EFI_GUID*, EFI_HANDLE, EFI_HANDLE);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE)     (EFI_LOCATE_SEARCH_TYPE, EFI_GUID*, VOID*, UINTN*, EFI_HANDLE*);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE_BUFFER)(EFI_LOCATE_SEARCH_TYPE, EFI_GUID*, VOID*, UINTN*, EFI_HANDLE**);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_DEVICE_PATH)(EFI_GUID*, EFI_DEVICE_PATH_PROTOCOL**, EFI_HANDLE*);
typedef EFI_STATUS (EFIAPI *EFI_OPEN_PROTOCOL_INFORMATION)(EFI_HANDLE, EFI_GUID*, EFI_OPEN_PROTOCOL_INFORMATION_ENTRY**, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_PROTOCOLS_PER_HANDLE)(EFI_HANDLE, EFI_GUID***, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_INSTALL_CONFIGURATION_TABLE)(EFI_GUID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_CALCULATE_CRC32)   (VOID*, UINTN, UINT32*);
typedef EFI_STATUS (EFIAPI *EFI_RAISE_TPL)         (EFI_TPL);
typedef EFI_STATUS (EFIAPI *EFI_RESTORE_TPL)       (EFI_TPL);
typedef EFI_STATUS (EFIAPI *EFI_STALL)             (UINTN);
typedef EFI_STATUS (EFIAPI *EFI_GET_NEXT_MONOTONIC_COUNT)(uint64_t*);
typedef EFI_STATUS (EFIAPI *EFI_EXIT)              (EFI_HANDLE, EFI_STATUS, UINTN, CHAR16*);
typedef VOID       (EFIAPI *EFI_EVENT_NOTIFY)      (EFI_EVENT, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_CREATE_EVENT)      (UINT32, EFI_TPL, EFI_EVENT_NOTIFY, VOID*, EFI_EVENT*);
typedef EFI_STATUS (EFIAPI *EFI_CREATE_EVENT_EX)   (UINT32, EFI_TPL, EFI_EVENT_NOTIFY, VOID*, EFI_GUID*, EFI_EVENT*);
typedef EFI_STATUS (EFIAPI *EFI_SET_TIMER)         (EFI_EVENT, EFI_TIMER_DELAY, uint64_t);
typedef EFI_STATUS (EFIAPI *EFI_WAIT_FOR_EVENT)    (UINTN, EFI_EVENT*, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_SIGNAL_EVENT)      (EFI_EVENT);
typedef EFI_STATUS (EFIAPI *EFI_CLOSE_EVENT)       (EFI_EVENT);
typedef EFI_STATUS (EFIAPI *EFI_CHECK_EVENT)       (EFI_EVENT);
typedef VOID       (EFIAPI *EFI_COPY_MEM)          (VOID*, VOID*, UINTN);
typedef VOID       (EFIAPI *EFI_SET_MEM)           (VOID*, UINTN, UINT8);

// ── boot services ─────────────────────────────────────────────────
#define EFI_BOOT_SERVICES_SIGNATURE  0x56524553544f4f42ULL
#define EFI_BOOT_SERVICES_REVISION   ((2 << 16) | 70)

typedef struct {
    EFI_TABLE_HEADER              Hdr;

    EFI_RAISE_TPL                 RaiseTPL;
    EFI_RESTORE_TPL               RestoreTPL;

    EFI_ALLOCATE_PAGES            AllocatePages;
    EFI_FREE_PAGES                FreePages;
    EFI_GET_MEMORY_MAP            GetMemoryMap;
    EFI_ALLOCATE_POOL             AllocatePool;
    EFI_FREE_POOL                 FreePool;

    EFI_CREATE_EVENT              CreateEvent;
    EFI_SET_TIMER                 SetTimer;
    EFI_WAIT_FOR_EVENT            WaitForEvent;
    EFI_SIGNAL_EVENT              SignalEvent;
    EFI_CLOSE_EVENT               CloseEvent;
    EFI_CHECK_EVENT               CheckEvent;

    void *InstallProtocolInterface;
    void *ReinstallProtocolInterface;
    void *UninstallProtocolInterface;
    EFI_HANDLE_PROTOCOL           HandleProtocol;
    void                          *Reserved;
    void *RegisterProtocolNotify;
    EFI_LOCATE_HANDLE             LocateHandle;
    EFI_LOCATE_DEVICE_PATH        LocateDevicePath;
    EFI_INSTALL_CONFIGURATION_TABLE InstallConfigurationTable;

    void *LoadImage;
    void *StartImage;
    EFI_EXIT                      Exit;
    void *UnloadImage;
    EFI_EXIT_BOOT_SERVICES        ExitBootServices;

    EFI_GET_NEXT_MONOTONIC_COUNT  GetNextMonotonicCount;
    EFI_STALL                     Stall;
    EFI_SET_WATCHDOG_TIMER        SetWatchdogTimer;

    void *ConnectController;
    void *DisconnectController;

    EFI_OPEN_PROTOCOL             OpenProtocol;
    EFI_CLOSE_PROTOCOL            CloseProtocol;
    EFI_OPEN_PROTOCOL_INFORMATION OpenProtocolInformation;
    EFI_PROTOCOLS_PER_HANDLE      ProtocolsPerHandle;
    EFI_LOCATE_HANDLE_BUFFER      LocateHandleBuffer;
    EFI_LOCATE_PROTOCOL           LocateProtocol;
    void *InstallMultipleProtocolInterfaces;
    void *UninstallMultipleProtocolInterfaces;

    EFI_CALCULATE_CRC32           CalculateCrc32;
    EFI_COPY_MEM                  CopyMem;
    EFI_SET_MEM                   SetMem;
    EFI_CREATE_EVENT_EX           CreateEventEx;
} EFI_BOOT_SERVICES;

// ── runtime services (mostly void* for now) ───────────────────────
#define EFI_RUNTIME_SERVICES_SIGNATURE  0x56524553544e5552ULL
#define EFI_RUNTIME_SERVICES_REVISION   ((2 << 16) | 70)

typedef struct {
    EFI_TABLE_HEADER  Hdr;
    void *GetTime, *SetTime, *GetWakeupTime, *SetWakeupTime;
    void *SetVirtualAddressMap, *ConvertPointer;
    void *GetVariable, *GetNextVariableName, *SetVariable;
    void *GetNextHighMonotonicCount;
    void *ResetSystem;
    void *UpdateCapsule, *QueryCapsuleCapabilities;
    void *QueryVariableInfo;
} EFI_RUNTIME_SERVICES;

// ── system table ──────────────────────────────────────────────────
#define EFI_SYSTEM_TABLE_SIGNATURE  0x5453595320494249ULL

typedef struct {
    EFI_TABLE_HEADER                Hdr;
    CHAR16                         *FirmwareVendor;
    uint32_t                        FirmwareRevision;
    uint32_t                        _pad;
    EFI_HANDLE                      ConsoleInHandle;
    void                           *ConIn;
    EFI_HANDLE                      ConsoleOutHandle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL *ConOut;
    EFI_HANDLE                      StandardErrorHandle;
    void                           *StdErr;
    EFI_RUNTIME_SERVICES           *RuntimeServices;
    EFI_BOOT_SERVICES              *BootServices;
    UINTN                           NumberOfTableEntries;
    void                           *ConfigurationTable;
} EFI_SYSTEM_TABLE;

typedef EFI_STATUS (EFIAPI *EFI_IMAGE_ENTRY_POINT)(EFI_HANDLE, EFI_SYSTEM_TABLE*);
#define EFI_INVALID_PARAMETER 0x8000000000000004   