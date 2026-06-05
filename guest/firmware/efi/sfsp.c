#include "sfsp.h"
#include "../headers/serial.h"
#include "../headers/memcmp.h"
#include "../headers/errors.h"
#include "../disk/esp.h"
#include "../disk/fat32.h"

/* ── Internal file state ──────────────────────────────────────── */
typedef struct {
    char     dos_name[11];
    uint32_t first_cluster;
    uint32_t file_size;
    uint32_t position;
    int      is_dir;
    int      in_use;
} FileState;

/* ── File protocol = state + vtable ───────────────────────────── */
typedef struct {
    FileState        state;
    EFI_FILE_PROTOCOL proto;
} FileHandle;

#define MAX_OPEN_FILES 6
static FileHandle gFilePool[MAX_OPEN_FILES];

/* Shared FAT32 handle for all SFS operations */
static Fat32_Handle gSfsFat;
static int gSfsFatValid = 0;
static int gSfsFatInitialized = 0;

/* ── EFI_FILE_INFO GUID (standard) ────────────────────────────── */
static EFI_GUID gEfiFileInfoGuid = {
    0x09576E92, 0x6D3F, 0x11D2,
    {0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

#define EFI_FILE_INFO_SIZE (sizeof(uint64_t) + sizeof(EFI_GUID) + sizeof(uint64_t) + sizeof(uint64_t) + sizeof(uint64_t) + sizeof(EFI_TIME) + sizeof(uint64_t) + 256*2)

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

/* ── Helper: convert CHAR16 path tail → 11-byte DOS name ──────── */
static CHAR16 to_upper_c(CHAR16 c) {
    if (c >= L'a' && c <= L'z') return c - (L'a' - L'A');
    return c;
}

static void name_to_dos(const CHAR16 *name, char *dos_out) {
    for (int i = 0; i < 11; i++) dos_out[i] = ' ';

    int len = 0;
    CHAR16 buf[256];
    while (name[len] && name[len] != L'\\' && len < 255) {
        buf[len] = to_upper_c(name[len]);
        len++;
    }
    buf[len] = 0;

    int dot = -1;
    for (int i = 0; i < len; i++) {
        if (buf[i] == L'.') { dot = i; break; }
    }

    if (dot < 0) {
        int copy = len < 8 ? len : 8;
        for (int i = 0; i < copy; i++) dos_out[i] = (char)buf[i];
    } else {
        int n = dot < 8 ? dot : 8;
        for (int i = 0; i < n; i++) dos_out[i] = (char)buf[i];
        int e = (len - dot - 1) < 3 ? (len - dot - 1) : 3;
        for (int i = 0; i < e; i++) dos_out[8 + i] = (char)buf[dot + 1 + i];
    }
}

/* ── Helper: ensure FAT32 is open ─────────────────────────────── */
static int ensure_fat(void) {
    if (gSfsFatValid) return 0;

    SectorRange range;
    int st = load_part_table(&range);
    if (st != 0) return st;

    st = open_fat32(&range, &gSfsFat);
    if (st != 0) return st;

    gSfsFatValid = 1;
    return 0;
}

/* ── Helper: find dir entry by DOS name in currently-loaded dir ─ */
static DirEntry *find_dos(const char *dos) {
    DirEntry *e;
    while (next_dir_entry(&gSfsFat, &e) == 0) {
        if (memcmp(e->name, dos, 11) == 0)
            return e;
    }
    return NULL;
}

/* ── Helper: walk a path, open the final entry ────────────────── */
/* Returns 0 on success (entry_out is set), non-zero on failure.   */
/* If path is "\" and we reach the root, *is_root = 1.            */
static DirEntry *walk_path(const CHAR16 *path, int *is_dir) {
    const CHAR16 *p = path;
    while (*p == L'\\') p++;

    if (ensure_fat() != 0) return NULL;
    if (open_root_dir(&gSfsFat) != 0) return NULL;

    if (*p == 0) {
        *is_dir = 1;
        return (DirEntry*)(uintptr_t)1; /* sentinel for root */
    }

    CHAR16 comp[256];
    while (*p) {
        int ci = 0;
        while (*p && *p != L'\\' && ci < 255)
            comp[ci++] = *p++;
        comp[ci] = 0;
        if (ci == 0) { if (*p) p++; continue; }

        char dos[11];
        name_to_dos(comp, dos);

        DirEntry *e = find_dos(dos);
        if (!e) return NULL;

        if (*p == L'\\') {
            if (!(e->attr & 0x10)) return NULL;
            if (open_dir_entry(&gSfsFat, e) != 0) return NULL;
            p++;
            continue;
        }
        /* end of path */
        *is_dir = (e->attr & 0x10) ? 1 : 0;
        return e;
    }

    return NULL;
}

/* ── Allocate a file handle slot ──────────────────────────────── */
static FileHandle *alloc_handle(void) {
    for (int i = 0; i < MAX_OPEN_FILES; i++) {
        if (!gFilePool[i].state.in_use) {
            gFilePool[i].state.in_use = 1;
            return &gFilePool[i];
        }
    }
    return NULL;
}

/* ── Forward declarations of file protocol methods ────────────── */
static EFI_STATUS EFIAPI file_Open(
    EFI_FILE_PROTOCOL *This, EFI_FILE_PROTOCOL **NewHandle,
    CHAR16 *FileName, UINT64 OpenMode, UINT64 Attributes);
static EFI_STATUS EFIAPI file_Close(EFI_FILE_PROTOCOL *This);
static EFI_STATUS EFIAPI file_Read(
    EFI_FILE_PROTOCOL *This, UINTN *BufferSize, VOID *Buffer);
static EFI_STATUS EFIAPI file_GetInfo(
    EFI_FILE_PROTOCOL *This, EFI_GUID *InfoType,
    UINTN *BufferSize, VOID *Buffer);
static EFI_STATUS EFIAPI file_SetPosition(
    EFI_FILE_PROTOCOL *This, UINT64 Position);

/* ── File protocol vtable ─────────────────────────────────────── */
static EFI_FILE_PROTOCOL gFileProtoTemplate = {
    .Revision    = EFI_FILE_PROTOCOL_REVISION2,
    .Open        = file_Open,
    .Close       = file_Close,
    .Delete      = NULL,
    .Read        = file_Read,
    .Write       = NULL,
    .GetPosition = NULL,
    .SetPosition = file_SetPosition,
    .GetInfo     = file_GetInfo,
    .SetInfo     = NULL,
    .Flush       = NULL,
};

/* ── Initialize a handle for root ─────────────────────────────── */
static void init_root_handle(FileHandle *fh) {
    fh->proto   = gFileProtoTemplate;
    fh->state.in_use = 1;
    for (int i = 0; i < 11; i++) fh->state.dos_name[i] = ' ';
    fh->state.first_cluster = 0;
    fh->state.file_size  = 0;
    fh->state.position   = 0;
    fh->state.is_dir     = 1;
}

/* ── File protocol implementations ────────────────────────────── */

static EFI_STATUS EFIAPI file_Open(
    EFI_FILE_PROTOCOL *This, EFI_FILE_PROTOCOL **NewHandle,
    CHAR16 *FileName, UINT64 OpenMode, UINT64 Attributes)
{
    (void)Attributes;
    if (!This || !NewHandle || !FileName)
        return EFI_INVALID_PARAMETER;
    if (OpenMode != EFI_FILE_MODE_READ)
        return EFI_UNSUPPORTED;

    serial_puts("[SFS] Open path=");
    for (int i = 0; FileName[i]; i++)
        serial_putc((char)FileName[i]);
    serial_puts("\n");

    /* Walk the path */
    int is_dir = 0;
    DirEntry *entry = walk_path(FileName, &is_dir);

    if (!entry) {
        serial_puts("[SFS] not found\n");
        return EFI_NOT_FOUND;
    }

    FileHandle *fh = alloc_handle();
    if (!fh) return EFI_OUT_OF_RESOURCES;

    fh->proto = gFileProtoTemplate;

    if ((uintptr_t)entry == 1) {
        /* Root directory */
        init_root_handle(fh);
        *NewHandle = &fh->proto;
        serial_puts("[SFS] opened root\n");
        return EFI_SUCCESS;
    }

    /* Copy entry info */
    for (int i = 0; i < 11; i++) fh->state.dos_name[i] = entry->name[i];
    fh->state.first_cluster = ((uint32_t)entry->first_cluster_high << 16) | entry->first_cluster_low;
    fh->state.file_size = entry->file_size;
    fh->state.position = 0;
    fh->state.is_dir = (entry->attr & 0x10) ? 1 : 0;

    serial_puts("[SFS] opened file size=0x");
    serial_putx(fh->state.file_size);
    serial_puts(" cluster=0x");
    serial_putx(fh->state.first_cluster);
    serial_puts("\n");

    *NewHandle = &fh->proto;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI file_Close(EFI_FILE_PROTOCOL *This) {
    if (!This) return EFI_INVALID_PARAMETER;

    /* Find the file handle */
    for (int i = 0; i < MAX_OPEN_FILES; i++) {
        if (&gFilePool[i].proto == This) {
            gFilePool[i].state.in_use = 0;
            serial_puts("[SFS] closed\n");
            return EFI_SUCCESS;
        }
    }
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI file_Read(
    EFI_FILE_PROTOCOL *This, UINTN *BufferSize, VOID *Buffer)
{
    if (!This || !BufferSize || !Buffer)
        return EFI_INVALID_PARAMETER;
    if (*BufferSize == 0) return EFI_SUCCESS;

    /* Find our state */
    FileHandle *fh = NULL;
    for (int i = 0; i < MAX_OPEN_FILES; i++) {
        if (&gFilePool[i].proto == This) {
            fh = &gFilePool[i];
            break;
        }
    }
    if (!fh) return EFI_INVALID_PARAMETER;

    if (fh->state.is_dir) {
        /* Directories: not supported for reading (could enumerate) */
        return EFI_UNSUPPORTED;
    }

    uint32_t remaining = fh->state.file_size - fh->state.position;
    if (*BufferSize < remaining) remaining = *BufferSize;
    if (remaining == 0) {
        *BufferSize = 0;
        return EFI_SUCCESS;
    }

    /* Use read_file_offset to read starting from file position.
     * The cluster offset is (position / cluster_size).
     * We need to read from the correct cluster.
     */
    uint32_t cluster_offset = fh->state.position / gSfsFat.cluster_size;
    uint32_t local_offset   = fh->state.position % gSfsFat.cluster_size;

    /* Compute the starting cluster */
    uint32_t cluster = fh->state.first_cluster;
    for (uint32_t i = 0; i < cluster_offset; i++) {
        uint32_t next;
        if (read_fat_entry(&gSfsFat, cluster, &next) != 0)
            return EFI_DEVICE_ERROR;
        if (next >= 0x0FFFFFF8) return EFI_DEVICE_ERROR;
        cluster = next;
    }

    uint32_t to_read = remaining;
    uint32_t offset  = 0;

    while (to_read > 0) {
        uint32_t lba = cluster_to_lba(&gSfsFat, cluster);
        uint32_t avail = gSfsFat.cluster_size - local_offset;
        uint32_t chunk = to_read < avail ? to_read : avail;

        /* Read from the sector-aligned offset within the cluster */
        uint32_t disk_offset = local_offset & ~511; // round down to sector
        uint32_t skip = local_offset - disk_offset; // bytes to skip in buffer

        /* Read one or more full sectors */
        uint32_t sector_aligned = ((chunk + skip + 511) & ~511);
        uint8_t tmp[8192];
        if (sector_aligned > sizeof(tmp))
            return EFI_BUFFER_TOO_SMALL;

        if (virtio_blk_read(
                gSfsFat.range->first_sector + lba + disk_offset / 512,
                sector_aligned, tmp) != 0)
            return EFI_DEVICE_ERROR;

        for (uint32_t i = 0; i < chunk; i++)
            ((uint8_t*)Buffer)[offset + i] = tmp[skip + i];

        offset += chunk;
        to_read -= chunk;
        local_offset = 0;

        if (to_read == 0) break;

        /* Next cluster */
        uint32_t next;
        if (read_fat_entry(&gSfsFat, cluster, &next) != 0)
            return EFI_DEVICE_ERROR;
        if (next >= 0x0FFFFFF8) break; // end of chain
        cluster = next;
    }

    fh->state.position += offset;
    *BufferSize = offset;
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI file_SetPosition(
    EFI_FILE_PROTOCOL *This, UINT64 Position)
{
    if (!This) return EFI_INVALID_PARAMETER;

    FileHandle *fh = NULL;
    for (int i = 0; i < MAX_OPEN_FILES; i++) {
        if (&gFilePool[i].proto == This) {
            fh = &gFilePool[i];
            break;
        }
    }
    if (!fh) return EFI_INVALID_PARAMETER;

    if (Position == 0xFFFFFFFFFFFFFFFFULL) {
        /* Seek to end */
        fh->state.position = fh->state.file_size;
    } else if (Position > fh->state.file_size) {
        return EFI_UNSUPPORTED;
    } else {
        fh->state.position = (uint32_t)Position;
    }
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI file_GetInfo(
    EFI_FILE_PROTOCOL *This, EFI_GUID *InfoType,
    UINTN *BufferSize, VOID *Buffer)
{
    if (!This || !InfoType || !BufferSize)
        return EFI_INVALID_PARAMETER;

    /* Only FileInfo is supported */
    if (!memcmp(InfoType, &gEfiFileInfoGuid, sizeof(EFI_GUID)))
        return EFI_UNSUPPORTED;

    FileHandle *fh = NULL;
    for (int i = 0; i < MAX_OPEN_FILES; i++) {
        if (&gFilePool[i].proto == This) {
            fh = &gFilePool[i];
            break;
        }
    }
    if (!fh) return EFI_INVALID_PARAMETER;

    EFI_FILE_INFO_FW info;
    UINTN need = sizeof(info);
    if (*BufferSize < need) {
        *BufferSize = need;
        return EFI_BUFFER_TOO_SMALL;
    }

    info.Size          = sizeof(info);
    info.FileSize      = fh->state.file_size;
    info.PhysicalSize  = ((fh->state.file_size + gSfsFat.cluster_size - 1) / gSfsFat.cluster_size) * gSfsFat.cluster_size;
    info.CreateTime    = 0;
    info.LastAccessTime = 0;
    info.ModificationTime = 0;
    info.Attribute     = fh->state.is_dir ? 0x10 : 0x20;

    /* Copy filename (convert DOS name back to CHAR16) */
    int pos = 0;
    /* Name part */
    int end = 7;
    while (end >= 0 && fh->state.dos_name[end] == ' ') end--;
    for (int i = 0; i <= end && i < 8; i++)
        info.FileName[pos++] = fh->state.dos_name[i];
    /* Extension */
    int ext_end = 10;
    while (ext_end >= 8 && fh->state.dos_name[ext_end] == ' ') ext_end--;
    if (ext_end >= 8) {
        info.FileName[pos++] = L'.';
        for (int i = 8; i <= ext_end && i < 11; i++)
            info.FileName[pos++] = fh->state.dos_name[i];
    }
    info.FileName[pos] = 0;

    /* Update size to match actual name length */
    UINTN actual_size = sizeof(EFI_FILE_INFO_FW) - 256*2 + (pos + 1) * 2;
    info.Size = actual_size;

    if (*BufferSize < actual_size) {
        // should not happen since we already checked against max
        *BufferSize = actual_size;
        return EFI_BUFFER_TOO_SMALL;
    }

    memcpy(Buffer, &info, actual_size);
    *BufferSize = actual_size;
    return EFI_SUCCESS;
}

/* ── OpenVolume ───────────────────────────────────────────────── */
static EFI_STATUS EFIAPI sfs_OpenVolume(
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *This,
    EFI_FILE_PROTOCOL **Root)
{
    (void)This;
    if (!Root) return EFI_INVALID_PARAMETER;

    serial_puts("[SFS] OpenVolume\n");

    if (!gSfsFatInitialized) {
        /* Open FAT32 once */
        int st = ensure_fat();
        if (st != 0) {
            serial_puts("[SFS] FAT32 init failed\n");
            return EFI_DEVICE_ERROR;
        }
        gSfsFatInitialized = 1;
    }

    /* Return the root handle */
    FileHandle *fh = alloc_handle();
    if (!fh) return EFI_OUT_OF_RESOURCES;
    init_root_handle(fh);

    *Root = &fh->proto;
    serial_puts("[SFS] OpenVolume -> root OK\n");
    return EFI_SUCCESS;
}

/* ── Global SFS protocol instance ─────────────────────────────── */
EFI_GUID gEfiSimpleFileSystemProtocolGuid = {
    0x0964E5B22, 0x6459, 0x11D2,
    {0x8E, 0x39, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B}
};

EFI_SIMPLE_FILE_SYSTEM_PROTOCOL gSfsp = {
    .Revision   = EFI_SIMPLE_FILE_SYSTEM_PROTOCOL_REVISION,
    .OpenVolume = sfs_OpenVolume,
};
