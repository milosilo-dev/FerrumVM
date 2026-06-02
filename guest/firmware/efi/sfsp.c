#include "sfsp.h"

typedef struct _EFI_FILE_PROTOCOL {
    UINT64 Revision;
    void* Open;
    void* Read;
    void* Close;
    void* GetInfo;
    void* SetInfo;
} EFI_FILE_PROTOCOL;

typedef EFI_STATUS (EFIAPI *EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME)(
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *This,
    EFI_FILE_PROTOCOL **Root
);

typedef struct _EFI_SIMPLE_FILE_SYSTEM_PROTOCOL {
    UINT64 Revision;
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_OPEN_VOLUME OpenVolume;
} EFI_SIMPLE_FILE_SYSTEM_PROTOCOL;

EFI_GUID gEfiSimpleFileSystemProtocolGuid = {
    0x0964E5B22, 0x6459, 0x11D2, 
        { 0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B }
};

EFI_FILE_PROTOCOL FakeRootFileProtocol = {
    .Revision = EFI_FILE_PROTOCOL_LATEST_REVISION,
};

EFI_STATUS EFIAPI OpenVolume(EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *This, EFI_FILE_PROTOCOL **Root)
{
    *Root = &FakeRootFileProtocol;
    return EFI_SUCCESS;
}

EFI_SIMPLE_FILE_SYSTEM_PROTOCOL gSfsp = {
    .Revision = EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION,
    .OpenVolume = OpenVolume
};