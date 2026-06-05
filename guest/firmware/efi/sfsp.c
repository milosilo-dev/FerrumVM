#include "sfsp.h"

/* ── EFI_FILE_INFO GUID (standard) ────────────────────────────── */
static EFI_GUID gEfiFileInfoGuid = {
    0x09576E92, 0x6D3F, 0x11D2,
    {0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

typedef struct {
    uint64_t Size;
    uint64_t FileSize;
    uint64_t PhysicalSize;
    uint64_t CreateTime;
    uint64_t LastAccessTime;
    uint64_t ModificationTime;
    uint64_t Attribute;
    CHAR16   FileName[256];
} __attribute__((packed)) EFI_FILE_INFO_FW;

/* ── File protocol vtable ─────────────────────────────────────── */
static EFI_FILE_PROTOCOL gFileProtoTemplate = {
    .Revision    = EFI_FILE_PROTOCOL_REVISION2,
    .Open        = NULL,
    .Close       = NULL,
    .Delete      = NULL,
    .Read        = NULL,
    .Write       = NULL,
    .GetPosition = NULL,
    .SetPosition = NULL,
    .GetInfo     = NULL,
    .SetInfo     = NULL,
    .Flush       = NULL,
};

/* ── Global SFS protocol instance ─────────────────────────────── */
EFI_GUID gEfiSimpleFileSystemProtocolGuid = {
    0x0964E5B22, 0x6459, 0x11D2,
    {0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

EFI_SIMPLE_FILE_SYSTEM_PROTOCOL gSfsp = {
    .Revision   = EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION,
    .OpenVolume = NULL,
};
