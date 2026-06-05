#pragma once
#include <stdint.h>
#include <stddef.h>

typedef int         BOOLEAN;  /* must match firmware's `int` for ABI compat */
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
typedef UINT64      EFI_PHYSICAL_ADDRESS;
typedef UINT64      EFI_VIRTUAL_ADDRESS;
typedef void*       EFI_HANDLE;
typedef void*       EFI_EVENT;
typedef UINT64      EFI_TPL;
typedef void        VOID;
typedef UINT16      CHAR16;

#define EFIAPI __attribute__((ms_abi))

#define EFI_SUCCESS             0ULL
#define EFI_ERROR(status)       ((status) & 0x8000000000000000ULL)
#define EFI_ERR(code)           (0x8000000000000000ULL | (code))
#define EFI_INVALID_PARAMETER   EFI_ERR(4)
#define EFI_BUFFER_TOO_SMALL    EFI_ERR(5)
#define EFI_NOT_READY           EFI_ERR(6)
#define EFI_DEVICE_ERROR        EFI_ERR(7)
#define EFI_OUT_OF_RESOURCES    EFI_ERR(9)
#define EFI_NOT_FOUND           EFI_ERR(14)
#define EFI_UNSUPPORTED         EFI_ERR(3)

typedef enum {
    AllocateAnyPages,
    AllocateMaxAddress,
    AllocateAddress,
    MaxAllocateType
} EFI_ALLOCATE_TYPE;

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

typedef enum {
    ByProtocol,
    ByHandleProtocol,
    ByRegisterNotify
} EFI_LOCATE_SEARCH_TYPE;

typedef enum {
    EFI_NATIVE_INTERFACE
} EFI_INTERFACE_TYPE;

typedef enum {
    TimerCancel,
    TimerPeriodic,
    TimerRelative
} EFI_TIMER_DELAY;

typedef enum {
    EFI_OPEN_PROTOCOL_BY_HANDLE_PROTOCOL  = 0x1,
    EFI_OPEN_PROTOCOL_GET_PROTOCOL        = 0x2,
    EFI_OPEN_PROTOCOL_TEST_PROTOCOL       = 0x4,
    EFI_OPEN_PROTOCOL_BY_CHILD_CONTROLLER = 0x8,
    EFI_OPEN_PROTOCOL_BY_DRIVER           = 0x10,
    EFI_OPEN_PROTOCOL_EXCLUSIVE           = 0x20
} EFI_OPEN_PROTOCOL_ATTRIBUTES;

typedef struct {
    UINT32 Type;
    UINT32 Pad;
    EFI_PHYSICAL_ADDRESS PhysicalStart;
    EFI_VIRTUAL_ADDRESS  VirtualStart;
    UINT64 NumberOfPages;
    UINT64 Attribute;
} EFI_MEMORY_DESCRIPTOR;

typedef struct {
    UINT32 Data1;
    UINT16 Data2;
    UINT16 Data3;
    UINT8  Data4[8];
} EFI_GUID;

typedef struct {
    UINT64 Signature;
    UINT32 Revision;
    UINT32 HeaderSize;
    UINT32 CRC32;
    UINT32 Reserved;
} EFI_TABLE_HEADER;

typedef struct {
    UINT8  Type;
    UINT8  SubType;
    UINT16 Length;
} EFI_DEVICE_PATH_PROTOCOL;

#define MAX_PATH_NODES 32
typedef struct {
    EFI_DEVICE_PATH_PROTOCOL *Nodes[MAX_PATH_NODES];
    UINTN                    Count;
} DEVICE_PATH_LIST;

/* ===== Simple Text Output Protocol ===== */
typedef struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL;
typedef struct {
    UINT32 MaxMode;
    UINT32 Mode;
    UINT32 Attribute;
    UINT32 CursorColumn;
    UINT32 CursorRow;
    BOOLEAN CursorVisible;
} SIMPLE_TEXT_OUTPUT_MODE;

typedef EFI_STATUS (EFIAPI *EFI_TEXT_RESET)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_STRING)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_TEST_STRING)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_QUERY_MODE)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, UINTN, UINTN*, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_SET_MODE)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_SET_ATTRIBUTE)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_CLEAR_SCREEN)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_SET_CURSOR_POS)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, UINTN, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_TEXT_ENABLE_CURSOR)(EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL*, BOOLEAN);

struct _EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL {
    EFI_TEXT_RESET            Reset;
    EFI_TEXT_STRING           OutputString;
    EFI_TEXT_TEST_STRING      TestString;
    EFI_TEXT_QUERY_MODE       QueryMode;
    EFI_TEXT_SET_MODE         SetMode;
    EFI_TEXT_SET_ATTRIBUTE    SetAttribute;
    EFI_TEXT_CLEAR_SCREEN     ClearScreen;
    EFI_TEXT_SET_CURSOR_POS   SetCursorPosition;
    EFI_TEXT_ENABLE_CURSOR    EnableCursor;
    SIMPLE_TEXT_OUTPUT_MODE  *Mode;
};

#define EFI_BLACK   0x00
#define EFI_BLUE    0x01
#define EFI_GREEN   0x02
#define EFI_CYAN    0x03
#define EFI_RED     0x04
#define EFI_MAGENTA 0x05
#define EFI_BROWN   0x06
#define EFI_LIGHTGRAY  0x07
#define EFI_BRIGHT_BLACK   0x08
#define EFI_BRIGHT_BLUE    0x09
#define EFI_BRIGHT_GREEN   0x0A
#define EFI_BRIGHT_CYAN    0x0B
#define EFI_BRIGHT_RED     0x0C
#define EFI_BRIGHT_MAGENTA 0x0D
#define EFI_YELLOW  0x0E
#define EFI_WHITE   0x0F

/* ===== Forward declarations for cross-referencing structs ===== */
typedef struct _EFI_SIMPLE_TEXT_INPUT_PROTOCOL EFI_SIMPLE_TEXT_INPUT_PROTOCOL;
typedef struct _EFI_SYSTEM_TABLE EFI_SYSTEM_TABLE;

typedef struct {
    UINT16 ScanCode;
    CHAR16 UnicodeChar;
} EFI_INPUT_KEY;

typedef EFI_STATUS (EFIAPI *EFI_INPUT_RESET)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_INPUT_READ_KEY)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL*, EFI_INPUT_KEY*);
typedef EFI_STATUS (EFIAPI *EFI_INPUT_WAIT_FOR_KEY)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL*, EFI_INPUT_KEY*);

struct _EFI_SIMPLE_TEXT_INPUT_PROTOCOL {
    EFI_INPUT_RESET     Reset;
    EFI_INPUT_READ_KEY  ReadKeyStroke;
    EFI_EVENT           WaitForKey;
};

/* ===== Loaded Image Protocol ===== */
#define EFI_LOADED_IMAGE_PROTOCOL_REVISION 0x1000

typedef EFI_STATUS (EFIAPI *EFI_IMAGE_UNLOAD)(EFI_HANDLE);

typedef struct {
    UINT32                    Revision;
    EFI_HANDLE                ParentHandle;
    EFI_SYSTEM_TABLE         *SystemTable;
    EFI_HANDLE                DeviceHandle;
    EFI_DEVICE_PATH_PROTOCOL *FilePath;
    VOID                     *Reserved;
    UINT32                    LoadOptionsSize;
    VOID                     *LoadOptions;
    VOID                     *ImageBase;
    UINT64                    ImageSize;
    EFI_MEMORY_TYPE           ImageCodeType;
    EFI_MEMORY_TYPE           ImageDataType;
    EFI_IMAGE_UNLOAD          Unload;
} EFI_LOADED_IMAGE_PROTOCOL;

/* ===== Device Path Protocol types ===== */
#define DEVICE_PATH_TYPE_HARDWARE_DEVICE 0x01
#define DEVICE_PATH_TYPE_ACPI_DEVICE     0x02
#define DEVICE_PATH_TYPE_MESSAGING_DEVICE 0x03
#define DEVICE_PATH_TYPE_MEDIA_DEVICE    0x04
#define DEVICE_PATH_TYPE_BIOS_BOOT_SPEC  0x05
#define DEVICE_PATH_TYPE_END             0x7F

#define DEVICE_PATH_SUBTYPE_END    0xFF
#define DEVICE_PATH_SUBTYPE_INSTANCE_END 0x01

#define DEVICE_PATH_SUBTYPE_HARDDRIVE 0x01
#define DEVICE_PATH_SUBTYPE_CDROM     0x02
#define DEVICE_PATH_SUBTYPE_FILE_PATH 0x04
#define DEVICE_PATH_SUBTYPE_MSG_SATA  0x12
#define DEVICE_PATH_SUBTYPE_MSG_NVME 0x17

/* ===== Block I/O Protocol ===== */
#define EFI_BLOCK_IO_PROTOCOL_REVISION 0x1001

typedef struct {
    UINT32  MediaId;
    BOOLEAN RemovableMedia;
    BOOLEAN MediaPresent;
    BOOLEAN LogicalPartition;
    BOOLEAN ReadOnly;
    BOOLEAN WriteCaching;
    UINT32  BlockSize;
    UINT32  IoAlign;
    UINTN   LastBlock;
} EFI_BLOCK_IO_MEDIA;

typedef struct _EFI_BLOCK_IO EFI_BLOCK_IO;

typedef EFI_STATUS (EFIAPI *EFI_BLOCK_RESET)(EFI_BLOCK_IO*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_BLOCK_READ)(EFI_BLOCK_IO*, UINT32, EFI_PHYSICAL_ADDRESS, UINTN, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_BLOCK_WRITE)(EFI_BLOCK_IO*, UINT32, EFI_PHYSICAL_ADDRESS, UINTN, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_BLOCK_FLUSH)(EFI_BLOCK_IO*);

struct _EFI_BLOCK_IO {
    UINT64           Revision;
    EFI_BLOCK_IO_MEDIA *Media;
    EFI_BLOCK_RESET  Reset;
    EFI_BLOCK_READ   ReadBlocks;
    EFI_BLOCK_WRITE  WriteBlocks;
    EFI_BLOCK_FLUSH  FlushBlocks;
};

/* ===== Simple File System Protocol ===== */
#define EFI_FILE_PROTOCOL_REVISION        0x00010000
#define EFI_FILE_PROTOCOL_LATEST_REVISION 0x00010000
#define EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION 0x00010000

typedef struct _EFI_FILE_PROTOCOL EFI_FILE_PROTOCOL;

typedef EFI_STATUS (EFIAPI *EFI_FILE_OPEN)(EFI_FILE_PROTOCOL*, EFI_FILE_PROTOCOL**, CHAR16*, UINT64, UINT64);
typedef EFI_STATUS (EFIAPI *EFI_FILE_CLOSE)(EFI_FILE_PROTOCOL*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_DELETE)(EFI_FILE_PROTOCOL*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_READ)(EFI_FILE_PROTOCOL*, UINTN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_WRITE)(EFI_FILE_PROTOCOL*, UINTN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_GET_POSITION)(EFI_FILE_PROTOCOL*, UINT64*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_SET_POSITION)(EFI_FILE_PROTOCOL*, UINT64);
typedef EFI_STATUS (EFIAPI *EFI_FILE_GET_INFO)(EFI_FILE_PROTOCOL*, EFI_GUID*, UINTN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_SET_INFO)(EFI_FILE_PROTOCOL*, EFI_GUID*, UINTN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_FILE_FLUSH)(EFI_FILE_PROTOCOL*);

struct _EFI_FILE_PROTOCOL {
    UINT64             Revision;
    EFI_FILE_OPEN      Open;
    EFI_FILE_CLOSE     Close;
    EFI_FILE_DELETE    Delete;
    EFI_FILE_READ      Read;
    EFI_FILE_WRITE     Write;
    EFI_FILE_GET_POSITION GetPosition;
    EFI_FILE_SET_POSITION SetPosition;
    EFI_FILE_GET_INFO  GetInfo;
    EFI_FILE_SET_INFO  SetInfo;
    EFI_FILE_FLUSH     Flush;
};

typedef struct _EFI_SIMPLE_FILE_SYSTEM_PROTOCOL EFI_SIMPLE_FILE_SYSTEM_PROTOCOL;

typedef EFI_STATUS (EFIAPI *EFI_OPEN_VOLUME)(EFI_SIMPLE_FILE_SYSTEM_PROTOCOL*, EFI_FILE_PROTOCOL**);

struct _EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    UINT64           Revision;
    EFI_OPEN_VOLUME  OpenVolume;
};

#define EFI_FILE_MODE_READ     0x0000000000000001ULL
#define EFI_FILE_MODE_WRITE    0x0000000000000002ULL
#define EFI_FILE_MODE_CREATE   0x8000000000000000ULL

/* ===== Event types ===== */
#define EVT_TIMER                           0x80000000
#define EVT_RUNTIME                         0x40000000
#define EVT_NOTIFY_WAIT                     0x00000100
#define EVT_NOTIFY_SIGNAL                   0x00000200
#define EVT_SIGNAL_EXIT_BOOT_SERVICES       0x00000201
#define EVT_SIGNAL_VIRTUAL_ADDRESS_CHANGE   0x60000202

#define EFI_EVENT_GROUP_EXIT_BOOT_SERVICES    { 0x27ABF055, 0xB1B8, 0x4C26, {0x80, 0x48, 0x74, 0x8F, 0x37, 0xBC, 0xA6, 0x85} }

/* ===== File System Info GUIDs ===== */
typedef struct {
    EFI_GUID VendorGuid;
    VOID    *VendorTable;
} EFI_CONFIGURATION_TABLE;

/* ===== Boot Services ===== */
typedef EFI_STATUS (EFIAPI *EFI_RAISE_TPL)(EFI_TPL);
typedef EFI_STATUS (EFIAPI *EFI_RESTORE_TPL)(EFI_TPL);
typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_PAGES)(EFI_ALLOCATE_TYPE, EFI_MEMORY_TYPE, UINTN, EFI_PHYSICAL_ADDRESS*);
typedef EFI_STATUS (EFIAPI *EFI_FREE_PAGES)(EFI_PHYSICAL_ADDRESS, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_GET_MEMORY_MAP)(UINTN*, VOID*, UINTN*, UINTN*, UINT32*);
typedef EFI_STATUS (EFIAPI *EFI_ALLOCATE_POOL)(EFI_MEMORY_TYPE, UINTN, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_FREE_POOL)(VOID*);
typedef EFI_STATUS (EFIAPI *EFI_CREATE_EVENT)(UINT32, EFI_TPL, VOID*, VOID*, EFI_EVENT*);
typedef EFI_STATUS (EFIAPI *EFI_SET_TIMER)(EFI_EVENT, EFI_TIMER_DELAY, UINT64);
typedef EFI_STATUS (EFIAPI *EFI_WAIT_FOR_EVENT)(UINTN, EFI_EVENT*, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_SIGNAL_EVENT)(EFI_EVENT);
typedef EFI_STATUS (EFIAPI *EFI_CLOSE_EVENT)(EFI_EVENT);
typedef EFI_STATUS (EFIAPI *EFI_CHECK_EVENT)(EFI_EVENT);
typedef EFI_STATUS (EFIAPI *EFI_INSTALL_PROTOCOL_INTERFACE)(EFI_HANDLE*, EFI_GUID*, VOID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_REINSTALL_PROTOCOL_INTERFACE)(EFI_HANDLE, EFI_GUID*, VOID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_UNINSTALL_PROTOCOL_INTERFACE)(EFI_HANDLE, EFI_GUID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_HANDLE_PROTOCOL)(EFI_HANDLE, EFI_GUID*, VOID**);
typedef VOID*       (EFIAPI *EFI_HANDLE_PROTOCOL_OLD)(EFI_HANDLE, EFI_GUID*, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_REGISTER_PROTOCOL_NOTIFY)(EFI_GUID*, EFI_EVENT, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE)(EFI_LOCATE_SEARCH_TYPE, EFI_GUID*, VOID*, UINTN*, EFI_HANDLE*);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_DEVICE_PATH)(EFI_GUID*, EFI_DEVICE_PATH_PROTOCOL**, EFI_HANDLE*);
typedef EFI_STATUS (EFIAPI *EFI_INSTALL_CONFIGURATION_TABLE)(EFI_GUID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_LOAD_IMAGE)(BOOLEAN, EFI_HANDLE, EFI_DEVICE_PATH_PROTOCOL*, VOID*, UINTN, EFI_HANDLE*);
typedef EFI_STATUS (EFIAPI *EFI_START_IMAGE)(EFI_HANDLE, UINTN*, CHAR16**);
typedef EFI_STATUS (EFIAPI *EFI_EXIT)(EFI_HANDLE, EFI_STATUS, UINTN, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_UNLOAD_IMAGE)(EFI_HANDLE);
typedef EFI_STATUS (EFIAPI *EFI_EXIT_BOOT_SERVICES)(EFI_HANDLE, UINTN);
typedef EFI_STATUS (EFIAPI *EFI_GET_NEXT_MONOTONIC_COUNT)(UINT64*);
typedef EFI_STATUS (EFIAPI *EFI_STALL)(UINTN);
typedef EFI_STATUS (EFIAPI *EFI_SET_WATCHDOG_TIMER)(UINTN, UINT64, UINTN, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_CONNECT_CONTROLLER)(EFI_HANDLE, EFI_HANDLE*, EFI_DEVICE_PATH_PROTOCOL*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_DISCONNECT_CONTROLLER)(EFI_HANDLE, EFI_HANDLE, EFI_HANDLE);
typedef EFI_STATUS (EFIAPI *EFI_OPEN_PROTOCOL)(EFI_HANDLE, EFI_GUID*, VOID**, EFI_HANDLE, EFI_HANDLE, UINT32);
typedef EFI_STATUS (EFIAPI *EFI_CLOSE_PROTOCOL)(EFI_HANDLE, EFI_GUID*, EFI_HANDLE, EFI_HANDLE);
typedef EFI_STATUS (EFIAPI *EFI_OPEN_PROTOCOL_INFORMATION)(EFI_HANDLE, EFI_GUID*, VOID**, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_PROTOCOLS_PER_HANDLE)(EFI_HANDLE, EFI_GUID***, UINTN*);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_HANDLE_BUFFER)(EFI_LOCATE_SEARCH_TYPE, EFI_GUID*, VOID*, UINTN*, EFI_HANDLE**);
typedef EFI_STATUS (EFIAPI *EFI_LOCATE_PROTOCOL)(EFI_GUID*, VOID*, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_INSTALL_MULTIPLE_PROTOCOL_INTERFACES)(EFI_HANDLE*, ...);
typedef EFI_STATUS (EFIAPI *EFI_UNINSTALL_MULTIPLE_PROTOCOL_INTERFACES)(EFI_HANDLE, ...);
typedef EFI_STATUS (EFIAPI *EFI_CALCULATE_CRC32)(VOID*, UINTN, UINT32*);
typedef VOID       (EFIAPI *EFI_COPY_MEM)(VOID*, VOID*, UINTN);
typedef VOID       (EFIAPI *EFI_SET_MEM)(VOID*, UINTN, UINT8);
typedef EFI_STATUS (EFIAPI *EFI_CREATE_EVENT_EX)(UINT32, EFI_TPL, VOID*, VOID*, EFI_GUID*, EFI_EVENT*);

typedef struct {
    EFI_TABLE_HEADER                   Hdr;
    EFI_RAISE_TPL                      RaiseTPL;
    EFI_RESTORE_TPL                    RestoreTPL;
    EFI_ALLOCATE_PAGES                 AllocatePages;
    EFI_FREE_PAGES                     FreePages;
    EFI_GET_MEMORY_MAP                 GetMemoryMap;
    EFI_ALLOCATE_POOL                  AllocatePool;
    EFI_FREE_POOL                      FreePool;
    EFI_CREATE_EVENT                   CreateEvent;
    EFI_SET_TIMER                      SetTimer;
    EFI_WAIT_FOR_EVENT                 WaitForEvent;
    EFI_SIGNAL_EVENT                   SignalEvent;
    EFI_CLOSE_EVENT                    CloseEvent;
    EFI_CHECK_EVENT                    CheckEvent;
    EFI_INSTALL_PROTOCOL_INTERFACE     InstallProtocolInterface;
    EFI_REINSTALL_PROTOCOL_INTERFACE   ReinstallProtocolInterface;
    EFI_UNINSTALL_PROTOCOL_INTERFACE   UninstallProtocolInterface;
    VOID                              *HandleProtocol;
    VOID                              *Reserved;
    EFI_REGISTER_PROTOCOL_NOTIFY       RegisterProtocolNotify;
    EFI_LOCATE_HANDLE                  LocateHandle;
    EFI_LOCATE_DEVICE_PATH             LocateDevicePath;
    EFI_INSTALL_CONFIGURATION_TABLE    InstallConfigurationTable;
    EFI_LOAD_IMAGE                     LoadImage;
    EFI_START_IMAGE                    StartImage;
    EFI_EXIT                           Exit;
    EFI_UNLOAD_IMAGE                   UnloadImage;
    EFI_EXIT_BOOT_SERVICES             ExitBootServices;
    EFI_GET_NEXT_MONOTONIC_COUNT       GetNextMonotonicCount;
    EFI_STALL                          Stall;
    EFI_SET_WATCHDOG_TIMER             SetWatchdogTimer;
    EFI_CONNECT_CONTROLLER             ConnectController;
    EFI_DISCONNECT_CONTROLLER          DisconnectController;
    EFI_OPEN_PROTOCOL                  OpenProtocol;
    EFI_CLOSE_PROTOCOL                 CloseProtocol;
    EFI_OPEN_PROTOCOL_INFORMATION       OpenProtocolInformation;
    EFI_PROTOCOLS_PER_HANDLE           ProtocolsPerHandle;
    EFI_LOCATE_HANDLE_BUFFER           LocateHandleBuffer;
    EFI_LOCATE_PROTOCOL                LocateProtocol;
    EFI_INSTALL_MULTIPLE_PROTOCOL_INTERFACES  InstallMultipleProtocolInterfaces;
    EFI_UNINSTALL_MULTIPLE_PROTOCOL_INTERFACES UninstallMultipleProtocolInterfaces;
    EFI_CALCULATE_CRC32                CalculateCrc32;
    EFI_COPY_MEM                       CopyMem;
    EFI_SET_MEM                        SetMem;
    EFI_CREATE_EVENT_EX                CreateEventEx;
} EFI_BOOT_SERVICES;

/* ===== Runtime Services ===== */
typedef EFI_STATUS (EFIAPI *EFI_GET_TIME)(VOID*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_SET_TIME)(VOID*);
typedef EFI_STATUS (EFIAPI *EFI_GET_WAKEUP_TIME)(BOOLEAN*, BOOLEAN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_SET_WAKEUP_TIME)(BOOLEAN, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_SET_VIRTUAL_ADDRESS_MAP)(UINTN, UINTN, UINT32, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_CONVERT_POINTER)(UINTN, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_GET_VARIABLE)(CHAR16*, EFI_GUID*, UINT32*, UINTN*, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_GET_NEXT_VARIABLE_NAME)(UINTN*, CHAR16*, EFI_GUID*);
typedef EFI_STATUS (EFIAPI *EFI_SET_VARIABLE)(CHAR16*, EFI_GUID*, UINT32, UINTN, VOID*);
typedef EFI_STATUS (EFIAPI *EFI_GET_NEXT_HIGH_MONOTONIC_COUNT)(UINT32*);
typedef EFI_STATUS (EFIAPI *EFI_RESET_SYSTEM)(BOOLEAN, EFI_STATUS, UINTN, CHAR16*);
typedef EFI_STATUS (EFIAPI *EFI_UPDATE_CAPSULE)(VOID**, VOID*, EFI_PHYSICAL_ADDRESS);
typedef EFI_STATUS (EFIAPI *EFI_QUERY_CAPSULE_CAPABILITIES)(EFI_GUID*, UINTN, UINT64*, VOID**);
typedef EFI_STATUS (EFIAPI *EFI_QUERY_VARIABLE_INFO)(UINT32, UINT64*, UINT64*, UINT64*);

typedef struct {
    EFI_TABLE_HEADER                     Hdr;
    EFI_GET_TIME                         GetTime;
    EFI_SET_TIME                         SetTime;
    EFI_GET_WAKEUP_TIME                  GetWakeupTime;
    EFI_SET_WAKEUP_TIME                  SetWakeupTime;
    EFI_SET_VIRTUAL_ADDRESS_MAP          SetVirtualAddressMap;
    EFI_CONVERT_POINTER                  ConvertPointer;
    EFI_GET_VARIABLE                     GetVariable;
    EFI_GET_NEXT_VARIABLE_NAME           GetNextVariableName;
    EFI_SET_VARIABLE                     SetVariable;
    EFI_GET_NEXT_HIGH_MONOTONIC_COUNT    GetNextHighMonotonicCount;
    EFI_RESET_SYSTEM                     ResetSystem;
    EFI_UPDATE_CAPSULE                   UpdateCapsule;
    EFI_QUERY_CAPSULE_CAPABILITIES       QueryCapsuleCapabilities;
    EFI_QUERY_VARIABLE_INFO              QueryVariableInfo;
} EFI_RUNTIME_SERVICES;

/* ===== System Table ===== */
struct _EFI_SYSTEM_TABLE {
    EFI_TABLE_HEADER                   Hdr;
    CHAR16                            *FirmwareVendor;
    UINT32                             FirmwareRevision;
    EFI_HANDLE                         ConsoleInHandle;
    VOID                              *ConIn;
    EFI_HANDLE                         ConsoleOutHandle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL   *ConOut;
    EFI_HANDLE                         StandardErrorHandle;
    EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL   *StdErr;
    EFI_RUNTIME_SERVICES              *RuntimeServices;
    EFI_BOOT_SERVICES                 *BootServices;
    UINTN                              NumberOfTableEntries;
    EFI_CONFIGURATION_TABLE           *ConfigurationTable;
};

typedef EFI_STATUS (EFIAPI *EFI_IMAGE_ENTRY_POINT)(EFI_HANDLE, EFI_SYSTEM_TABLE*);

/* ===== Well-known GUIDs ===== */
#define GUID_EFI_LOADED_IMAGE_PROTOCOL \
    { 0x5B1B31A1, 0x9562, 0x11D2, { 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }
#define GUID_EFI_DEVICE_PATH_PROTOCOL \
    { 0x09576E91, 0x6D3F, 0x11D2, { 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }
#define GUID_EFI_BLOCK_IO_PROTOCOL \
    { 0x964E5B21, 0x6459, 0x11D2, { 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }
#define GUID_EFI_SIMPLE_FILE_SYSTEM_PROTOCOL \
    { 0x0964E5B22, 0x6459, 0x11D2, { 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }
#define GUID_EFI_ACPI_20_TABLE \
    { 0x8868E871, 0xE4F1, 0x11D3, { 0xBC, 0x22, 0x00, 0x80, 0xC7, 0x3C, 0x88, 0x81 } }
#define GUID_EFI_ACPI_TABLE \
    { 0xEB9D2D30, 0x2D88, 0x11D3, { 0x9A, 0x16, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D } }
#define GUID_EFI_FILE_SYSTEM_INFO \
    { 0x09576E93, 0x6D3F, 0x11D2, { 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }
#define GUID_EFI_FILE_SYSTEM_VOLUME_LABEL \
    { 0xDB47D7D3, 0xFE81, 0x11D3, { 0x9A, 0x1B, 0x00, 0x90, 0x27, 0x3F, 0xC1, 0x4D } }

/* ===== Dbg helpers ===== */
static inline int guid_eq(const EFI_GUID *a, const EFI_GUID *b) {
    if (a->Data1 != b->Data1) return 0;
    if (a->Data2 != b->Data2) return 0;
    if (a->Data3 != b->Data3) return 0;
    for (int i = 0; i < 8; i++)
        if (a->Data4[i] != b->Data4[i]) return 0;
    return 1;
}

static inline UINT64 guid_hash(EFI_GUID *g) {
    return g->Data1 ^ ((UINT64)g->Data2 << 16) ^ ((UINT64)g->Data3 << 32);
}
