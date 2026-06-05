#pragma once
#include "../headers/uefi/uefi.h"

#define EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION    0x00010000
#define EFI_FILE_PROTOCOL_REVISION2                 0x00020000
#define EFI_FILE_PROTOCOL_LATEST_REVISION           EFI_FILE_PROTOCOL_REVISION2

typedef struct _EFI_SIMPLE_FILE_SYSTEM_PROTOCOL EFI_SIMPLE_FILE_SYSTEM_PROTOCOL;
typedef struct _EFI_FILE_PROTOCOL EFI_FILE_PROTOCOL;

typedef EFI_STATUS (EFIAPI *EFI_FILE_OPEN)(
    EFI_FILE_PROTOCOL *This,
    EFI_FILE_PROTOCOL **NewHandle,
    CHAR16 *FileName,
    UINT64 OpenMode,
    UINT64 Attributes
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_CLOSE)(EFI_FILE_PROTOCOL *This);
typedef EFI_STATUS (EFIAPI *EFI_FILE_DELETE)(EFI_FILE_PROTOCOL *This);

typedef EFI_STATUS (EFIAPI *EFI_FILE_READ)(
    EFI_FILE_PROTOCOL *This,
    UINTN *BufferSize,
    VOID *Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_WRITE)(
    EFI_FILE_PROTOCOL *This,
    UINTN *BufferSize,
    VOID *Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_GET_POSITION)(
    EFI_FILE_PROTOCOL *This,
    UINT64 *Position
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_SET_POSITION)(
    EFI_FILE_PROTOCOL *This,
    UINT64 Position
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_GET_INFO)(
    EFI_FILE_PROTOCOL *This,
    EFI_GUID *InformationType,
    UINTN *BufferSize,
    VOID *Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_SET_INFO)(
    EFI_FILE_PROTOCOL *This,
    EFI_GUID *InformationType,
    UINTN BufferSize,
    VOID *Buffer
);

typedef EFI_STATUS (EFIAPI *EFI_FILE_FLUSH)(EFI_FILE_PROTOCOL *This);

#define EFI_FILE_MODE_READ   0x0000000000000001ULL
#define EFI_FILE_MODE_WRITE  0x0000000000000002ULL
#define EFI_FILE_MODE_CREATE 0x8000000000000000ULL

struct _EFI_FILE_PROTOCOL {
    UINT64                Revision;
    EFI_FILE_OPEN         Open;
    EFI_FILE_CLOSE        Close;
    EFI_FILE_DELETE       Delete;
    EFI_FILE_READ         Read;
    EFI_FILE_WRITE        Write;
    EFI_FILE_GET_POSITION GetPosition;
    EFI_FILE_SET_POSITION SetPosition;
    EFI_FILE_GET_INFO     GetInfo;
    EFI_FILE_SET_INFO     SetInfo;
    EFI_FILE_FLUSH        Flush;
    VOID                 *OpenEx;
    VOID                 *ReadEx;
    VOID                 *WriteEx;
    VOID                 *FlushEx;
};

typedef EFI_STATUS (EFIAPI *EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME)(
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *This,
    EFI_FILE_PROTOCOL **Root
);

struct _EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    UINT64                                      Revision;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME OpenVolume;
};