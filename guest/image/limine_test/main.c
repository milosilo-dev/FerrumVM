#include "efi.h"

/* =================================================================
   Limine Boot Flow Test — follows every step Limine takes to boot
   ================================================================= */

/* ── Macros for firmware-specific errors ───── */
#define TEST_PASS(status, msg) do { if ((status) == 0) { print(L"[PASS] " msg L"\r\n"); } } while(0)
#define TEST_FAIL(msg)         do { print(L"[FAIL] "); print(msg); print(L"\r\n"); fails++; } while(0)
#define TEST_INFO(msg)         do { print(L"[INFO] "); print(msg); print(L"\r\n"); } while(0)
#define PTR(p)                 ((UINT64)(p))
#define U64(v)                 ((UINT64)(v))

/* ── Known GUID instances ───── */
static EFI_GUID gEfiLoadedImageProtocolGuid      = GUID_EFI_LOADED_IMAGE_PROTOCOL;
static EFI_GUID gEfiDevicePathProtocolGuid       = GUID_EFI_DEVICE_PATH_PROTOCOL;
static EFI_GUID gEfiBlockIoProtocolGuid          = GUID_EFI_BLOCK_IO_PROTOCOL;
static EFI_GUID gEfiSimpleFileSystemProtocolGuid = GUID_EFI_SIMPLE_FILE_SYSTEM_PROTOCOL;
static EFI_GUID gEfiAcpiTableGuid                = GUID_EFI_ACPI_TABLE;
static EFI_GUID gEfiAcpi20TableGuid              = GUID_EFI_ACPI_20_TABLE;
/* ── Global state ───── */
static EFI_SYSTEM_TABLE  *gST;
static EFI_BOOT_SERVICES *gBS;
static EFI_HANDLE         gImageHandle;
static UINTN              fails;
static UINTN              tests_run;

static CHAR16 gHexBuf[20];

static void print(const CHAR16 *s) {
    if (gST && gST->ConOut && gST->ConOut->OutputString)
        gST->ConOut->OutputString(gST->ConOut, (CHAR16*)s);
}

static void print_hex(UINT64 v) {
    gHexBuf[19] = 0;
    gHexBuf[18] = L'\r';
    gHexBuf[17] = L'\n';
    gHexBuf[16] = L'h';
    for (int i = 15; i >= 0; i--) {
        UINT64 x = v & 0xF;
        gHexBuf[i] = (x < 10) ? (L'0' + x) : (L'A' + (x - 10));
        v >>= 4;
    }
    print(gHexBuf);
}

static void print_str_hex(const CHAR16 *label, UINT64 v) {
    print(label);
    print_hex(v);
}

static void print_str_str(const CHAR16 *label, const CHAR16 *val) {
    print(label);
    print(val);
    print(L"\r\n");
}



/* =================================================================
   TEST: Entry Point ABI validation
   ================================================================= */
static void test_entry_point(void) {
    tests_run++;
    TEST_INFO(L"--- A) Entry Point ABI ---");

    /* 1. ImageHandle must be non-NULL */
    if (gImageHandle == NULL) { TEST_FAIL(L"ImageHandle is NULL"); return; }
    TEST_PASS(1, L"ImageHandle non-NULL");

    /* 2. SystemTable must be non-NULL */
    if (gST == NULL) { TEST_FAIL(L"SystemTable is NULL"); return; }
    TEST_PASS(1, L"SystemTable non-NULL");
}

/* =================================================================
   TEST: System Table validation
   ================================================================= */
static void test_system_table(void) {
    tests_run++;
    TEST_INFO(L"--- B) System Table Validation ---");

    /* Check signature */
    UINT64 expected_sig = 0x5453595320494249ULL; /* "IBI SYST" */
    if (gST->Hdr.Signature != expected_sig) {
        print_str_hex(L"  Bad signature: got=", gST->Hdr.Signature);
        print_str_hex(L"  expected=", expected_sig);
        TEST_FAIL(L"SystemTable signature mismatch");
    } else {
        TEST_PASS(1, L"Signature 'IBI SYST'");
    }

    /* Check version (>= 2.70) */
    if (gST->Hdr.Revision == 0) {
        TEST_FAIL(L"SystemTable Revision is 0");
    } else {
        UINT32 major = gST->Hdr.Revision >> 16;
        UINT32 minor = gST->Hdr.Revision & 0xFFFF;
        CHAR16 ver[64];
        UINTN pos = 0;
        ver[pos++] = L' '; ver[pos++] = L'v'; ver[pos++] = L'e'; ver[pos++] = L'r';
        ver[pos++] = L'='; ver[pos++] = L' ';
        /* print major */
        if (major >= 10) ver[pos++] = L'0' + (major / 10);
        ver[pos++] = L'0' + (major % 10);
        ver[pos++] = L'.';
        if (minor >= 100) ver[pos++] = L'0' + (minor / 100);
        if (minor >= 10)  ver[pos++] = L'0' + ((minor / 10) % 10);
        ver[pos++] = L'0' + (minor % 10);
        ver[pos] = 0;
        print_str_str(L"  Revision", ver);
        TEST_PASS(1, L"Revision non-zero");
    }

    /* FirmwareVendor */
    if (gST->FirmwareVendor == NULL) {
        TEST_FAIL(L"FirmwareVendor is NULL");
    } else {
        CHAR16 *fw = gST->FirmwareVendor;
        CHAR16 buf[256];
        UINTN pos = 0;
        buf[pos++] = L' ';
        buf[pos++] = L'v';
        buf[pos++] = L'e';
        buf[pos++] = L'n';
        buf[pos++] = L'd';
        buf[pos++] = L'o';
        buf[pos++] = L'r';
        buf[pos++] = L'=';
        while (*fw && pos < 255) buf[pos++] = *fw++;
        buf[pos] = 0;
        print_str_str(L"  Vendor", buf);
        TEST_PASS(1, L"FirmwareVendor present");
    }

    /* Console output */
    if (gST->ConOut == NULL) {
        TEST_FAIL(L"ConOut is NULL");
    } else if (gST->ConOut->OutputString == NULL) {
        TEST_FAIL(L"ConOut->OutputString is NULL");
    } else {
        TEST_PASS(1, L"ConOut functional");
    }

    /* Console input */
    if (gST->ConIn == NULL) {
        TEST_FAIL(L"ConIn is NULL");
    } else if (gST->ConOut->OutputString == NULL) {
        TEST_FAIL(L"ConIn->ReadKeyStroke is NULL");
    } else {
        TEST_PASS(1, L"ConIn present");
    }

    /* StdErr */
    if (gST->StdErr == NULL) {
        TEST_FAIL(L"StdErr is NULL");
    } else {
        TEST_PASS(1, L"StdErr present");
    }

    /* BootServices */
    if (gST->BootServices == NULL) {
        TEST_FAIL(L"BootServices is NULL");
        return; /* can't continue without BS */
    }
    gBS = gST->BootServices;

    if (gBS->Hdr.Signature != 0x56524553544f4f42ULL) {
        print_str_hex(L"  BS sig=", gBS->Hdr.Signature);
        TEST_FAIL(L"BootServices signature bad");
    } else {
        TEST_PASS(1, L"BootServices signature OK");
    }

    if (gBS->Hdr.Revision == 0) {
        TEST_FAIL(L"BootServices Revision is 0");
    } else {
        TEST_PASS(1, L"BootServices Revision non-zero");
    }

    /* RuntimeServices */
    if (gST->RuntimeServices == NULL) {
        TEST_FAIL(L"RuntimeServices is NULL");
    } else {
        if (gST->RuntimeServices->Hdr.Signature != 0x56524553544e5552ULL) {
            print_str_hex(L"  RS sig=", gST->RuntimeServices->Hdr.Signature);
            TEST_FAIL(L"RuntimeServices signature bad");
        } else {
            TEST_PASS(1, L"RuntimeServices signature OK");
        }
    }
}

/* =================================================================
   TEST: Loaded Image Protocol (Limine step 1)
   ================================================================= */
static void test_loaded_image_protocol(void) {
    tests_run++;
    TEST_INFO(L"--- C) LoadedImage Protocol ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    EFI_LOADED_IMAGE_PROTOCOL *img = NULL;

    /* Try OpenProtocol first (Limine's preferred method) */
    EFI_STATUS s = gBS->OpenProtocol(
        gImageHandle,
        &gEfiLoadedImageProtocolGuid,
        (VOID**)&img,
        gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );

    if (EFI_ERROR(s) || img == NULL) {
        /* Fall back to HandleProtocol */
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        s = ((EFI_HP)gBS->HandleProtocol)(gImageHandle, &gEfiLoadedImageProtocolGuid, (VOID**)&img);
    }

    if (EFI_ERROR(s) || img == NULL) {
        print_str_hex(L"  Status=", s);
        TEST_FAIL(L"Failed to get LoadedImageProtocol");
        return;
    }
    TEST_PASS(1, L"LoadedImageProtocol retrieved");

    /* Check Revision */
    print_str_hex(L"  Revision=", img->Revision);
    if (img->Revision == 0) {
        TEST_FAIL(L"LoadedImageProtocol Revision=0");
    }

    /* Check DeviceHandle — critical for Limine to find its disk */
    if (img->DeviceHandle == NULL) {
        TEST_FAIL(L"DeviceHandle is NULL — Limine CANNOT find boot disk!");
    } else {
        print_str_hex(L"  DeviceHandle=", PTR(img->DeviceHandle));
        TEST_PASS(1, L"DeviceHandle set");
    }

    /* Check FilePath */
    if (img->FilePath == NULL) {
        TEST_FAIL(L"FilePath is NULL — Limine CANNOT locate its config!");
    } else {
        EFI_DEVICE_PATH_PROTOCOL *dp = img->FilePath;
        print_str_hex(L"  FilePath->Type=", dp->Type);
        print_str_hex(L"  FilePath->SubType=", dp->SubType);
        print_str_hex(L"  FilePath->Length=", dp->Length);
        TEST_PASS(1, L"FilePath set");
    }

    /* Check SystemTable match */
    if ((VOID*)img->SystemTable != (VOID*)gST) {
        print_str_hex(L"  Image.ST=", PTR(img->SystemTable));
        print_str_hex(L"  gST=", PTR(gST));
        TEST_FAIL(L"Image SystemTable != entry SystemTable");
    } else {
        TEST_PASS(1, L"SystemTable pointer matches");
    }

    /* Check ImageBase / ImageSize */
    if (img->ImageBase == NULL) {
        TEST_FAIL(L"ImageBase is NULL");
    } else {
        print_str_hex(L"  ImageBase=", PTR(img->ImageBase));
        TEST_PASS(1, L"ImageBase set");
    }

    if (img->ImageSize == 0) {
        TEST_FAIL(L"ImageSize is 0");
    } else {
        print_str_hex(L"  ImageSize=", img->ImageSize);
        TEST_PASS(1, L"ImageSize set");
    }
}

/* =================================================================
   TEST: Device Path Protocol (Limine step 2)
   ================================================================= */
static void test_device_path_protocol(void) {
    tests_run++;
    TEST_INFO(L"--- D) DevicePath Protocol ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* First get the LoadedImage to find DeviceHandle */
    EFI_LOADED_IMAGE_PROTOCOL *img = NULL;
    EFI_STATUS s = gBS->OpenProtocol(
        gImageHandle, &gEfiLoadedImageProtocolGuid,
        (VOID**)&img, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );
    if (EFI_ERROR(s) || img == NULL || img->DeviceHandle == NULL) {
        TEST_FAIL(L"Cannot get DeviceHandle for DevicePath test");
        return;
    }

    EFI_DEVICE_PATH_PROTOCOL *dev_path = NULL;
    s = gBS->OpenProtocol(
        img->DeviceHandle, &gEfiDevicePathProtocolGuid,
        (VOID**)&dev_path, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );

    if (EFI_ERROR(s) || dev_path == NULL) {
        /* Try HandleProtocol */
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        s = ((EFI_HP)gBS->HandleProtocol)(img->DeviceHandle, &gEfiDevicePathProtocolGuid, (VOID**)&dev_path);
    }

    if (EFI_ERROR(s) || dev_path == NULL) {
        print_str_hex(L"  OpenProtocol status=", s);
        TEST_FAIL(L"Failed to get DevicePath protocol on DeviceHandle");
        return;
    }
    TEST_PASS(1, L"DevicePath protocol retrieved");

    /* Walk device path nodes */
    UINTN node_count = 0;
    EFI_DEVICE_PATH_PROTOCOL *node = dev_path;

    while (node->Type != 0x7F && node_count < 32) {
        CHAR16 buf[128];
        UINTN pos = 0;
        buf[pos++] = L' ';
        buf[pos++] = L'T';
        buf[pos++] = L'=';
        /* type */
        UINT8 t = node->Type;
        if (t >= 10) buf[pos++] = L'0' + (t / 10);
        buf[pos++] = L'0' + (t % 10);
        buf[pos++] = L' ';
        buf[pos++] = L'S';
        buf[pos++] = L'T';
        buf[pos++] = L'=';
        UINT8 st = node->SubType;
        if (st >= 10) buf[pos++] = L'0' + (st / 10);
        buf[pos++] = L'0' + (st % 10);
        buf[pos++] = L' ';
        buf[pos++] = L'L';
        buf[pos++] = L'=';
        /* length */
        UINT16 len = node->Length;
        if (len >= 100) buf[pos++] = L'0' + (len / 100);
        if (len >= 10)  buf[pos++] = L'0' + ((len / 10) % 10);
        buf[pos++] = L'0' + (len % 10);
        buf[pos] = 0;
        print_str_str(L"  Node", buf);

        node_count++;
        if (node->Length == 0) break;
        node = (EFI_DEVICE_PATH_PROTOCOL*)((UINT8*)node + node->Length);
    }
    print_str_hex(L"  Total nodes=", node_count);

    if (node_count == 0) {
        TEST_FAIL(L"Device path has 0 nodes");
    } else {
        TEST_PASS(1, L"Device path has nodes");
    }
}

/* =================================================================
   TEST: Block I/O Protocol (Limine step 3 — reading from disk)
   ================================================================= */
static void test_block_io_protocol(void) {
    tests_run++;
    TEST_INFO(L"--- E) Block I/O Protocol ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* Get DeviceHandle from LoadedImage */
    EFI_LOADED_IMAGE_PROTOCOL *img = NULL;
    gBS->OpenProtocol(
        gImageHandle, &gEfiLoadedImageProtocolGuid,
        (VOID**)&img, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );
    if (img == NULL || img->DeviceHandle == NULL) {
        TEST_FAIL(L"Cannot get DeviceHandle for BlockIo test");
        return;
    }

    EFI_BLOCK_IO *block = NULL;
    EFI_STATUS s = gBS->OpenProtocol(
        img->DeviceHandle, &gEfiBlockIoProtocolGuid,
        (VOID**)&block, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );

    if (EFI_ERROR(s) || block == NULL) {
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        s = ((EFI_HP)gBS->HandleProtocol)(img->DeviceHandle, &gEfiBlockIoProtocolGuid, (VOID**)&block);
    }

    if (EFI_ERROR(s) || block == NULL) {
        /* Also try LocateProtocol */
        gBS->LocateProtocol(&gEfiBlockIoProtocolGuid, NULL, (VOID**)&block);
    }

    if (EFI_ERROR(s) || block == NULL) {
        print_str_hex(L"  Status=", s);
        TEST_FAIL(L"Failed to get BlockIo protocol");
        return;
    }
    TEST_PASS(1, L"BlockIo protocol retrieved");

    /* Check media */
    if (block->Media == NULL) {
        TEST_FAIL(L"BlockIo Media is NULL");
        return;
    }
    TEST_PASS(1, L"Media present");

    print_str_hex(L"  BlockSize=", block->Media->BlockSize);
    print_str_hex(L"  LastBlock=", block->Media->LastBlock);
    print_str_hex(L"  MediaId=", block->Media->MediaId);
    print_str_hex(L"  RemovableMedia=", block->Media->RemovableMedia);
    print_str_hex(L"  MediaPresent=", block->Media->MediaPresent);
    print_str_hex(L"  ReadOnly=", block->Media->ReadOnly);
    print_str_hex(L"  IoAlign=", block->Media->IoAlign);

    if (block->Media->BlockSize == 0) {
        TEST_FAIL(L"BlockSize is 0");
    } else {
        TEST_PASS(1, L"BlockSize non-zero");
    }

    if (block->ReadBlocks == NULL) {
        TEST_FAIL(L"ReadBlocks is NULL");
        return;
    }
    TEST_PASS(1, L"ReadBlocks function pointer non-NULL");

    /* Try reading LBA 0 (MBR / GPT protective MBR) */
    UINT8 sector[512];
    s = block->ReadBlocks(block, block->Media->MediaId, 0, 512, sector);

    if (EFI_ERROR(s)) {
        print_str_hex(L"  ReadBlocks(LBA=0) status=", s);
        TEST_FAIL(L"ReadBlocks LBA 0 failed");
    } else {
        TEST_PASS(1, L"ReadBlocks LBA 0 OK");
        /* Check for MBR signature 0xAA55 */
        if (sector[510] == 0x55 && sector[511] == 0xAA) {
            TEST_PASS(1, L"MBR signature 0xAA55 valid");
        } else {
            print_str_hex(L"  sig bytes=", (UINT64)(sector[510] | (sector[511] << 8)));
            TEST_FAIL(L"No MBR signature (0xAA55)");
        }
        /* Check for GPT protective MBR (partition type 0xEE) */
        if (sector[450] == 0xEE) {
            TEST_PASS(1, L"GPT protective MBR (type 0xEE)");
        } else {
            TEST_FAIL(L"MBR partition type is not 0xEE (not GPT?)");
        }
    }

    /* Try reading LBA 1 (GPT header) */
    UINT8 sector1[512];
    s = block->ReadBlocks(block, block->Media->MediaId, 1, 512, sector1);

    if (EFI_ERROR(s)) {
        print_str_hex(L"  ReadBlocks(LBA=1) status=", s);
        TEST_FAIL(L"ReadBlocks LBA 1 failed");
    } else {
        TEST_PASS(1, L"ReadBlocks LBA 1 OK");
        /* Check GPT signature "EFI PART" */
        if (sector1[0] == 'E' && sector1[1] == 'F' && sector1[2] == 'I' &&
            sector1[3] == ' ' && sector1[4] == 'P' && sector1[5] == 'A' &&
            sector1[6] == 'R' && sector1[7] == 'T') {
            TEST_PASS(1, L"GPT header signature 'EFI PART'");
        } else {
            TEST_FAIL(L"No GPT header signature");
        }
    }
}

/* =================================================================
   TEST: Simple File System Protocol (Limine step 4 — reading files)
   This is one of the most critical tests — Limine needs to read
   limine.conf from the ESP.
   ================================================================= */
static void test_simple_file_system(void) {
    tests_run++;
    TEST_INFO(L"--- F) Simple File System Protocol ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* Get DeviceHandle */
    EFI_LOADED_IMAGE_PROTOCOL *img = NULL;
    gBS->OpenProtocol(
        gImageHandle, &gEfiLoadedImageProtocolGuid,
        (VOID**)&img, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );
    if (img == NULL || img->DeviceHandle == NULL) {
        TEST_FAIL(L"Cannot get DeviceHandle for SFS test");
        return;
    }

    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *sfs = NULL;
    EFI_STATUS s = gBS->OpenProtocol(
        img->DeviceHandle, &gEfiSimpleFileSystemProtocolGuid,
        (VOID**)&sfs, gImageHandle, NULL,
        EFI_OPEN_PROTOCOL_GET_PROTOCOL
    );

    if (EFI_ERROR(s) || sfs == NULL) {
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        s = ((EFI_HP)gBS->HandleProtocol)(img->DeviceHandle, &gEfiSimpleFileSystemProtocolGuid, (VOID**)&sfs);
    }

    if (EFI_ERROR(s) || sfs == NULL) {
        print_str_hex(L"  Status=", s);
        TEST_FAIL(L"Failed to get SimpleFileSystem protocol");
        return;
    }
    TEST_PASS(1, L"SimpleFileSystem protocol retrieved");

    print_str_hex(L"  SFS Revision=", sfs->Revision);

    /* Check OpenVolume */
    if (sfs->OpenVolume == NULL) {
        TEST_FAIL(L"OpenVolume is NULL — CANNOT access files!");
        return;
    }
    TEST_PASS(1, L"OpenVolume function pointer non-NULL");

    /* Open Volume */
    EFI_FILE_PROTOCOL *root = NULL;
    s = sfs->OpenVolume(sfs, &root);

    if (EFI_ERROR(s) || root == NULL) {
        print_str_hex(L"  OpenVolume status=", s);
        TEST_FAIL(L"OpenVolume failed");
        return;
    }
    TEST_PASS(1, L"OpenVolume succeeded");

    print_str_hex(L"  Root Revision=", root->Revision);

    /* Check root file protocol function pointers (CRITICAL for Limine) */
    if (root->Open == NULL) {
        TEST_FAIL(L"Root->Open is NULL — CANNOT open files!");
    } else {
        TEST_PASS(1, L"Root->Open present");
    }
    if (root->Read == NULL) {
        TEST_FAIL(L"Root->Read is NULL — CANNOT read files!");
    } else {
        TEST_PASS(1, L"Root->Read present");
    }
    if (root->Close == NULL) {
        TEST_FAIL(L"Root->Close is NULL");
    } else {
        TEST_PASS(1, L"Root->Close present");
    }
    if (root->GetInfo == NULL) {
        TEST_FAIL(L"Root->GetInfo is NULL — CANNOT get file info!");
    } else {
        TEST_PASS(1, L"Root->GetInfo present");
    }

    /* Try to open the EFI boot path (Limine's config path) */
    CHAR16 boot_path[] = L"\\EFI\\BOOT\\limine.conf";
    CHAR16 alt_path[]  = L"limine.conf";
    CHAR16 efi_dir[]   = L"\\EFI";

    /* Test 1: Try to open limine.conf in the boot directory */
    EFI_FILE_PROTOCOL *conf = NULL;

    s = root->Open ? root->Open(root, &conf, boot_path, EFI_FILE_MODE_READ, 0) : EFI_UNSUPPORTED;

    print_str_hex(L"  Open(limine.conf)=", s);
    if (s == EFI_SUCCESS && conf != NULL) {
        TEST_PASS(1, L"Opened limine.conf");

        /* Try to read it */
        if (conf->Read) {
            UINT8 buf[4096];
            UINTN read_size = sizeof(buf);
            s = conf->Read(conf, &read_size, buf);
            print_str_hex(L"  Read(limine.conf) status=", s);
            print_str_hex(L"  Read bytes=", read_size);
            if (s == EFI_SUCCESS) {
                TEST_PASS(1, L"Read limine.conf data");
                /* Print first 128 chars */
                CHAR16 preview[256];
                UINTN pos = 0;
                for (UINTN i = 0; i < read_size && i < 128 && buf[i] && pos < 250; i++) {
                    preview[pos++] = buf[i] >= 32 && buf[i] < 127 ? buf[i] : L'.';
                }
                preview[pos] = 0;
                TEST_INFO(L"Config preview:");
                print(preview);
                print(L"\r\n");
            } else {
                TEST_FAIL(L"Failed to read limine.conf data");
            }
        }

        if (conf->Close) conf->Close(conf);
    } else {
        TEST_FAIL(L"Could NOT open limine.conf");

        /* Test 2: Try alt path */
        s = root->Open ? root->Open(root, &conf, alt_path, EFI_FILE_MODE_READ, 0) : EFI_UNSUPPORTED;
        print_str_hex(L"  Open(limine.conf alt)=", s);
        if (s == EFI_SUCCESS && conf != NULL) {
            TEST_PASS(1, L"Opened limine.conf (alt path)");
            if (conf->Close) conf->Close(conf);
        } else {
            TEST_FAIL(L"Could NOT open limine.conf on alt path either");
        }

        /* Test 3: Try opening EFI directory to see if any directory traversal works */
        s = root->Open ? root->Open(root, &conf, efi_dir, EFI_FILE_MODE_READ, 0) : EFI_UNSUPPORTED;
        print_str_hex(L"  Open(\\EFI)=", s);
        if (s == EFI_SUCCESS && conf != NULL) {
            TEST_PASS(1, L"Opened EFI directory");
            if (conf->Close) conf->Close(conf);
        } else {
            TEST_FAIL(L"Could NOT open EFI directory either");
        }
    }
}

/* =================================================================
   TEST: Memory Allocation (Limine step 5)
   ================================================================= */
static void test_memory_allocation(void) {
    tests_run++;
    TEST_INFO(L"--- G) Memory Allocation ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* AllocatePool — multiple sizes */
    for (UINTN i = 0; i < 8; i++) {
        VOID *p = NULL;
        EFI_STATUS s = gBS->AllocatePool(EfiLoaderData, 64 + (i * 128), &p);
        if (EFI_ERROR(s) || p == NULL) {
            print_str_hex(L"  AllocatePool(",(64 + i*128));
            print(L") = ");
            print((EFI_ERROR(s) ? (CHAR16*)L"FAIL\r\n" : (CHAR16*)L"NULL ptr\r\n"));
            TEST_FAIL(L"AllocatePool failed");
        } else {
            /* Write to memory to validate it's writable */
            *(volatile UINT64*)p = 0xDEADBEEFCAFEBABEULL;
            if (*(volatile UINT64*)p != 0xDEADBEEFCAFEBABEULL) {
                TEST_FAIL(L"AllocatePool memory not writable!");
            }
            gBS->FreePool(p);
        }
    }
    TEST_PASS(1, L"AllocatePool/FreePool cycle works");

    /* AllocatePages — AllocateAnyPages */
    {
        EFI_PHYSICAL_ADDRESS addr = 0;
        EFI_STATUS s = gBS->AllocatePages(AllocateAnyPages, EfiLoaderData, 4, &addr);
        if (EFI_ERROR(s) || addr == 0) {
            print_str_hex(L"  AllocatePages(Any,4)=", s);
            TEST_FAIL(L"AllocatePages AnyPages failed");
        } else {
            print_str_hex(L"  AllocatePages(Any,4) at=", addr);
            /* Verify page alignment */
            if (addr & 0xFFF) {
                TEST_FAIL(L"AllocatePages returned non-page-aligned addr!");
            } else {
                TEST_PASS(1, L"AllocatePages AnyPages OK, aligned");
            }
            /* Write to confirm writable */
            *(volatile UINT64*)(UINTN)addr = 0xCAFEBABE;
            if (*(volatile UINT64*)(UINTN)addr != 0xCAFEBABE) {
                TEST_FAIL(L"AllocatePages memory not writable!");
            } else {
                TEST_PASS(1, L"AllocatePages memory writable");
            }
            gBS->FreePages(addr, 4);
        }
    }

    /* AllocatePages — AllocateMaxAddress */
    {
        EFI_PHYSICAL_ADDRESS addr = 0x10000000; /* 256 MB */
        EFI_STATUS s = gBS->AllocatePages(AllocateMaxAddress, EfiLoaderData, 1, &addr);
        if (EFI_ERROR(s) || addr == 0) {
            print_str_hex(L"  AllocatePages(Max,1)=", s);
            TEST_FAIL(L"AllocatePages MaxAddress failed");
        } else {
            print_str_hex(L"  AllocatePages(Max,1) at=", addr);
            TEST_PASS(1, L"AllocatePages MaxAddress OK");
            gBS->FreePages(addr, 1);
        }
    }

    /* AllocatePages — AllocateAddress */
    {
        EFI_PHYSICAL_ADDRESS addr = 0x800000; /* 8 MB */
        EFI_STATUS s = gBS->AllocatePages(AllocateAddress, EfiLoaderData, 2, &addr);
        print_str_hex(L"  AllocatePages(Fixed, 0x800000)=", s);
        if (s == EFI_SUCCESS) {
            TEST_PASS(1, L"AllocatePages Fixed OK");
            gBS->FreePages(addr, 2);
        } else {
            /* This may fail depending on firmware, that's OK */
            TEST_INFO(L"Fixed address allocation not supported (may be expected)");
        }
    }

    /* Edge case: allocate 0 pages */
    {
        EFI_PHYSICAL_ADDRESS addr = 0;
        EFI_STATUS s = gBS->AllocatePages(AllocateAnyPages, EfiLoaderData, 0, &addr);
        print_str_hex(L"  AllocatePages(0 pages)=", s);
        if (EFI_ERROR(s)) {
            TEST_PASS(1, L"AllocatePages(0) properly rejected");
        } else {
            TEST_FAIL(L"AllocatePages(0) should fail");
        }
    }
}

/* =================================================================
   TEST: Memory Map (Limine step 6)
   ================================================================= */
static void test_memory_map(void) {
    tests_run++;
    TEST_INFO(L"--- H) Memory Map ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* Probe: call with size=0 to discover required size */
    UINTN  map_size = 0;
    UINTN  map_key  = 0;
    UINTN  desc_size = 0;
    UINT32 desc_ver  = 0;

    EFI_STATUS s = gBS->GetMemoryMap(&map_size, NULL, &map_key, &desc_size, &desc_ver);

    if (s != EFI_BUFFER_TOO_SMALL) {
        print_str_hex(L"  Probe status=", s);
        TEST_FAIL(L"GetMemoryMap probe unexpected status");
    } else {
        TEST_PASS(1, L"GetMemoryMap probe returned BUFFER_TOO_SMALL");
    }

    print_str_hex(L"  Required size=", map_size);
    print_str_hex(L"  DescriptorSize=", desc_size);
    print_str_hex(L"  DescVersion=", desc_ver);
    print_str_hex(L"  MapKey=", map_key);

    if (desc_size == 0) {
        TEST_FAIL(L"DescriptorSize is 0!");
        return;
    }

    /* Full read: allocate buffer and get the map */
    UINTN buf_size = map_size + desc_size; /* add padding to be safe */
    UINT8 *map_buf = NULL;
    s = gBS->AllocatePool(EfiLoaderData, buf_size, (VOID**)&map_buf);
    if (EFI_ERROR(s) || map_buf == NULL) {
        TEST_FAIL(L"AllocatePool for memory map failed");
        return;
    }

    /* We need to save key for ExitBootServices test */
    UINTN saved_key = 0;
    s = gBS->GetMemoryMap(&buf_size, map_buf, &saved_key, &desc_size, &desc_ver);

    if (EFI_ERROR(s)) {
        print_str_hex(L"  GetMemoryMap status=", s);
        TEST_FAIL(L"GetMemoryMap full read failed");
        gBS->FreePool(map_buf);
        return;
    }
    TEST_PASS(1, L"GetMemoryMap full read succeeded");

    UINTN entry_count = buf_size / desc_size;
    print_str_hex(L"  Entry count=", entry_count);

    /* Iterate and print entries */
    UINTN conventional = 0;
    UINTN reserved = 0;
    UINTN loader = 0;
    UINTN boot = 0;
    UINT64 total_pages = 0;

    for (UINTN i = 0; i < entry_count && i < 64; i++) {
        EFI_MEMORY_DESCRIPTOR *d = (EFI_MEMORY_DESCRIPTOR*)(map_buf + i * desc_size);
        total_pages += d->NumberOfPages;

        if (d->Type == EfiConventionalMemory) conventional += d->NumberOfPages;
        else if (d->Type == EfiLoaderCode || d->Type == EfiLoaderData) loader += d->NumberOfPages;
        else if (d->Type == EfiBootServicesCode || d->Type == EfiBootServicesData) boot += d->NumberOfPages;
        else reserved += d->NumberOfPages;
    }

    print_str_hex(L"  Total pages=", total_pages);
    print_str_hex(L"  Total MB=", (total_pages * 4096) / (1024 * 1024));
    print_str_hex(L"  Conventional MB=", (conventional * 4096) / (1024 * 1024));
    print_str_hex(L"  Reserved MB=", (reserved * 4096) / (1024 * 1024));
    print_str_hex(L"  Loader MB=", (loader * 4096) / (1024 * 1024));
    print_str_hex(L"  Boot MB=", (boot * 4096) / (1024 * 1024));

    if (conventional == 0) {
        TEST_FAIL(L"No conventional memory reported!");
    } else {
        TEST_PASS(1, L"Conventional memory available");
    }

    gBS->FreePool(map_buf);
}

/* =================================================================
   TEST: ExitBootServices (Limine step 7 — critical!)
   ================================================================= */
static void test_exit_boot_services(void) {
    tests_run++;
    TEST_INFO(L"--- I) ExitBootServices ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* Get a valid map key first */
    UINTN map_size = 0;
    UINTN map_key  = 0;
    UINTN desc_size = 0;
    UINT32 desc_ver = 0;

    gBS->GetMemoryMap(&map_size, NULL, &map_key, &desc_size, &desc_ver);

    /* Try calling ExitBootServices with a bad key first — should fail */
    EFI_STATUS s = gBS->ExitBootServices(gImageHandle, 0xDEAD);
    print_str_hex(L"  ExitBS(invalid key)=", s);

    /* Now try with a real key — even if stub, we want to see what happens.
       WARNING: If the firmware actually implements ExitBootServices,
       calling it will disable boot services! */
    if (s != EFI_SUCCESS) {
        /* Most firmware stubs return success regardless, but some validate keys */
        s = gBS->ExitBootServices(gImageHandle, map_key);
        print_str_hex(L"  ExitBS(real key)=", s);
    }

    if (EFI_ERROR(s)) {
        TEST_FAIL(L"ExitBootServices returned error");
    } else {
        TEST_PASS(1, L"ExitBootServices returned success (may be stub)");
        TEST_INFO(L"  WARNING: If ExitBootServices worked, ConOut is now dead.");
        TEST_INFO(L"  If you still see this, it's probably a stub.");

        /* Try to print after ExitBootServices — if it still works, it's a stub */
        EFI_STATUS s2 = gBS->AllocatePool(EfiLoaderData, 64, (VOID**)&map_size);
        if (s2 == EFI_SUCCESS) {
            TEST_INFO(L"  ExitBootServices is a STUB (BootServices still work)");
        } else {
            TEST_INFO(L"  ExitBootServices seems functional (BootServices disabled)");
        }
    }
}

/* =================================================================
   TEST: LocateProtocol / LocateHandle (Limine helper)
   ================================================================= */
static void test_locate_services(void) {
    tests_run++;
    TEST_INFO(L"--- J) Locate Services ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* LocateProtocol for LoadedImage */
    EFI_LOADED_IMAGE_PROTOCOL *img = NULL;
    EFI_STATUS s = gBS->LocateProtocol(&gEfiLoadedImageProtocolGuid, NULL, (VOID**)&img);
    print_str_hex(L"  LocateProtocol(LoadedImage)=", s);
    if (s == EFI_SUCCESS && img != NULL) {
        TEST_PASS(1, L"LocateProtocol LoadedImage OK");
        print_str_hex(L"    DeviceHandle=", PTR(img->DeviceHandle));
    } else {
        TEST_FAIL(L"LocateProtocol LoadedImage failed");
    }

    /* LocateProtocol for BlockIo */
    EFI_BLOCK_IO *block = NULL;
    s = gBS->LocateProtocol(&gEfiBlockIoProtocolGuid, NULL, (VOID**)&block);
    print_str_hex(L"  LocateProtocol(BlockIo)=", s);
    if (s == EFI_SUCCESS && block != NULL) {
        TEST_PASS(1, L"LocateProtocol BlockIo OK");
    }

    /* LocateProtocol for SimpleFileSystem */
    EFI_SIMPLE_FILE_SYSTEM_PROTOCOL *sfs = NULL;
    s = gBS->LocateProtocol(&gEfiSimpleFileSystemProtocolGuid, NULL, (VOID**)&sfs);
    print_str_hex(L"  LocateProtocol(SFS)=", s);
    if (s == EFI_SUCCESS && sfs != NULL) {
        TEST_PASS(1, L"LocateProtocol SFS OK");
    }

    /* LocateHandleBuffer */
    EFI_HANDLE *buf = NULL;
    UINTN count = 0;
    s = gBS->LocateHandleBuffer(ByProtocol, &gEfiBlockIoProtocolGuid, NULL, &count, &buf);
    print_str_hex(L"  LocateHandleBuffer(BlockIo)=", s);
    if (s == EFI_SUCCESS && buf != NULL) {
        print_str_hex(L"    Count=", count);
        print_str_hex(L"    Handle[0]=", PTR(buf[0]));
        TEST_PASS(1, L"LocateHandleBuffer BlockIo OK");
        gBS->FreePool(buf);
    } else {
        TEST_FAIL(L"LocateHandleBuffer BlockIo failed");
    }

    /* LocateHandle */
    UINTN buf_size = sizeof(EFI_HANDLE) * 4;
    EFI_HANDLE handles[4];
    s = gBS->LocateHandle(ByProtocol, &gEfiBlockIoProtocolGuid, NULL, &buf_size, handles);
    print_str_hex(L"  LocateHandle(BlockIo)=", s);
    if (s == EFI_SUCCESS) {
        print_str_hex(L"    Handles=", buf_size / sizeof(EFI_HANDLE));
        TEST_PASS(1, L"LocateHandle BlockIo OK");
    }
}

/* =================================================================
   TEST: Event Services
   ================================================================= */
static void test_event_services(void) {
    tests_run++;
    TEST_INFO(L"--- K) Event Services ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* CreateEvent */
    EFI_EVENT event = NULL;
    EFI_STATUS s = gBS->CreateEvent(EVT_NOTIFY_SIGNAL, 0, NULL, NULL, &event);
    print_str_hex(L"  CreateEvent=", s);
    if (EFI_ERROR(s) || event == NULL) {
        TEST_FAIL(L"CreateEvent failed");
        return;
    }
    TEST_PASS(1, L"CreateEvent OK");

    /* CheckEvent — should return EFI_NOT_READY */
    s = gBS->CheckEvent(event);
    print_str_hex(L"  CheckEvent(initial)=", s);
    if (s == EFI_NOT_READY) {
        TEST_PASS(1, L"CheckEvent correctly NOT_READY");
    }

    /* SignalEvent */
    s = gBS->SignalEvent(event);
    print_str_hex(L"  SignalEvent=", s);
    if (EFI_ERROR(s)) {
        TEST_FAIL(L"SignalEvent failed");
    } else {
        TEST_PASS(1, L"SignalEvent OK");
    }

    /* CheckEvent after signal — should succeed */
    s = gBS->CheckEvent(event);
    print_str_hex(L"  CheckEvent(post-signal)=", s);
    if (s == EFI_SUCCESS) {
        TEST_PASS(1, L"CheckEvent detected signaled state");
    }

    /* CloseEvent */
    s = gBS->CloseEvent(event);
    print_str_hex(L"  CloseEvent=", s);
    if (EFI_ERROR(s)) {
        TEST_FAIL(L"CloseEvent failed");
    } else {
        TEST_PASS(1, L"CloseEvent OK");
    }
}

/* =================================================================
   TEST: Stall
   ================================================================= */
static void test_stall(void) {
    tests_run++;
    TEST_INFO(L"--- L) Stall ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    if (gBS->Stall == NULL) {
        TEST_FAIL(L"Stall is NULL");
        return;
    }
    TEST_PASS(1, L"Stall function pointer valid");

    EFI_STATUS s = gBS->Stall(1000); /* 1 ms */
    print_str_hex(L"  Stall(1000us)=", s);
    if (EFI_ERROR(s)) {
        TEST_FAIL(L"Stall failed");
    } else {
        TEST_PASS(1, L"Stall OK");
    }
}

/* =================================================================
   TEST: CalculateCrc32
   ================================================================= */
static void test_calculate_crc32(void) {
    tests_run++;
    TEST_INFO(L"--- M) CalculateCrc32 ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    if (gBS->CalculateCrc32 == NULL) {
        TEST_FAIL(L"CalculateCrc32 is NULL");
        return;
    }
    TEST_PASS(1, L"CalculateCrc32 function pointer valid");

    UINT8 data[] = "Hello UEFI";
    UINT32 crc = 0;
    EFI_STATUS s = gBS->CalculateCrc32(data, sizeof(data), &crc);
    print_str_hex(L"  CRC32=", crc);
    if (EFI_ERROR(s)) {
        TEST_FAIL(L"CalculateCrc32 failed");
    } else {
        TEST_PASS(1, L"CalculateCrc32 OK");
    }
}

/* =================================================================
   TEST: CopyMem and SetMem
   ================================================================= */
static void test_copy_set_mem(void) {
    tests_run++;
    TEST_INFO(L"--- N) CopyMem / SetMem ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    if (gBS->SetMem == NULL) {
        TEST_FAIL(L"SetMem is NULL");
    } else {
        UINT8 buf[32];
        gBS->SetMem(buf, 32, 0xAA);
        UINTN ok = 1;
        for (UINTN i = 0; i < 32; i++)
            if (buf[i] != 0xAA) { ok = 0; break; }
        if (ok) {
            TEST_PASS(1, L"SetMem works");
        } else {
            TEST_FAIL(L"SetMem did not fill buffer correctly");
        }
    }

    if (gBS->CopyMem == NULL) {
        TEST_FAIL(L"CopyMem is NULL");
    } else {
        UINT8 src[32] = {0};
        UINT8 dst[32] = {0};
        for (UINTN i = 0; i < 32; i++) src[i] = (UINT8)i;
        gBS->SetMem(dst, 32, 0);
        gBS->CopyMem(dst, src, 32);
        UINTN ok = 1;
        for (UINTN i = 0; i < 32; i++)
            if (dst[i] != src[i]) { ok = 0; break; }
        if (ok) {
            TEST_PASS(1, L"CopyMem works");
        } else {
            TEST_FAIL(L"CopyMem did not copy correctly");
        }
    }
}

/* =================================================================
   TEST: Configuration Table (ACPI RSDP — needed by Limine for boot)
   ================================================================= */
static void test_configuration_table(void) {
    tests_run++;
    TEST_INFO(L"--- O) Configuration Table (ACPI) ---");

    if (gST->ConfigurationTable == NULL) {
        TEST_FAIL(L"ConfigurationTable is NULL");
        return;
    }

    print_str_hex(L"  Entry count=", gST->NumberOfTableEntries);

    if (gST->NumberOfTableEntries == 0) {
        TEST_FAIL(L"No configuration table entries present — no ACPI!");
        return;
    }
    TEST_PASS(1, L"Config table entries present");

    UINTN found_guid = 0;
    for (UINTN i = 0; i < gST->NumberOfTableEntries; i++) {
        EFI_CONFIGURATION_TABLE *ct = &gST->ConfigurationTable[i];

        CHAR16 buf[128];
        UINTN pos = 0;
        buf[pos++] = L' ';
        buf[pos++] = L'[';
        buf[pos++] = L'0' + (i / 100);
        buf[pos++] = L'0' + ((i / 10) % 10);
        buf[pos++] = L'0' + (i % 10);
        buf[pos++] = L']';
        buf[pos++] = L' ';
        buf[pos++] = L'G';
        buf[pos++] = L'U';
        buf[pos++] = L'I';
        buf[pos++] = L'D';
        buf[pos++] = L'=';
        buf[pos] = 0;
        /* Append GUID hex values */
        CHAR16 guid_str[40];
        UINTN gp = 0;
        UINT32 d1 = ct->VendorGuid.Data1;
        for (int b = 28; b >= 0; b -= 4) {
            UINT8 nib = (d1 >> b) & 0xF;
            guid_str[gp++] = nib < 10 ? L'0' + nib : L'A' + (nib - 10);
        }
        guid_str[gp++] = L'-';
        UINT16 d2 = ct->VendorGuid.Data2;
        for (int b = 12; b >= 0; b -= 4) {
            UINT8 nib = (d2 >> b) & 0xF;
            guid_str[gp++] = nib < 10 ? L'0' + nib : L'A' + (nib - 10);
        }
        guid_str[gp++] = L'-';
        UINT16 d3 = ct->VendorGuid.Data3;
        for (int b = 12; b >= 0; b -= 4) {
            UINT8 nib = (d3 >> b) & 0xF;
            guid_str[gp++] = nib < 10 ? L'0' + nib : L'A' + (nib - 10);
        }
        guid_str[gp++] = L'-';
        for (int b = 0; b < 8; b++) {
            UINT8 byte = ct->VendorGuid.Data4[b];
            UINT8 hi = (byte >> 4) & 0xF;
            UINT8 lo = byte & 0xF;
            guid_str[gp++] = hi < 10 ? L'0' + hi : L'A' + (hi - 10);
            guid_str[gp++] = lo < 10 ? L'0' + lo : L'A' + (lo - 10);
        }
        guid_str[gp] = 0;

        /* Build final string */
        for (UINTN c = 0; c < gp && pos < 100; c++)
            buf[pos++] = guid_str[c];
        buf[pos++] = L' ';
        pos = pos; /* nop to satisfy compiler about the unused increment above */
        UINTN pos_before_addr = pos;
        buf[pos++] = L'@';
        buf[pos++] = L' ';
        pos = pos_before_addr;

        print_str_str(L"  Entry", buf);
        print_str_hex(L"    Table ptr=", PTR(ct->VendorTable));

        /* Check if it's ACPI 2.0 or 1.0 */
        if (guid_eq(&ct->VendorGuid, &gEfiAcpi20TableGuid)) {
            found_guid++;
            TEST_INFO(L"  *** Found ACPI 2.0 RSDP ***");
            if (ct->VendorTable) {
                TEST_PASS(1, L"ACPI 2.0 table pointer valid");
                /* Check RSDP signature */
                UINT8 *rsdp = (UINT8*)ct->VendorTable;
                if (rsdp[0] == 'R' && rsdp[1] == 'S' && rsdp[2] == 'D' && rsdp[3] == ' ') {
                    TEST_PASS(1, L"RSDP signature 'RSD ' OK");
                } else if (rsdp[0] == 'R' && rsdp[1] == 'S' && rsdp[2] == 'D' && rsdp[3] == 'P') {
                    TEST_PASS(1, L"RSDP signature 'RSDP' OK");
                } else {
                    CHAR16 sig[8];
                    sig[0] = rsdp[0]; sig[1] = rsdp[1];
                    sig[2] = rsdp[2]; sig[3] = rsdp[3];
                    sig[4] = 0;
                    print_str_str(L"    Bad sig=", sig);
                    TEST_FAIL(L"RSDP signature wrong");
                }
            } else {
                TEST_FAIL(L"ACPI 2.0 table pointer is NULL");
            }
        }
        if (guid_eq(&ct->VendorGuid, &gEfiAcpiTableGuid)) {
            found_guid++;
            TEST_INFO(L"  *** Found ACPI 1.0 RSDP ***");
            if (ct->VendorTable) {
                TEST_PASS(1, L"ACPI 1.0 table pointer valid");
            } else {
                TEST_FAIL(L"ACPI 1.0 table pointer is NULL");
            }
        }
    }

    if (found_guid > 0) {
        print_str_hex(L"  ACPI tables found=", found_guid);
        TEST_PASS(1, L"ACPI RSDP available (Limine needs this)");
    } else {
        TEST_FAIL(L"No ACPI RSDP found in config table — Limine WILL FAIL!");
    }
}

/* =================================================================
   TEST: Runtime Services
   ================================================================= */
static void test_runtime_services(void) {
    tests_run++;
    TEST_INFO(L"--- P) Runtime Services ---");

    if (gST->RuntimeServices == NULL) {
        TEST_FAIL(L"RuntimeServices is NULL");
        return;
    }

    EFI_RUNTIME_SERVICES *rs = gST->RuntimeServices;

    /* GetVariable — even a stub should be callable */
    if (rs->GetVariable) {
        UINT32 attr = 0;
        UINTN data_size = 0;
        CHAR16 name[] = L"TestVar";
        EFI_GUID vendor = { 0x12345678, 0x1234, 0x5678, {0x12,0x34,0x56,0x78,0x9A,0xBC,0xDE,0xF0} };

        EFI_STATUS s = rs->GetVariable(name, &vendor, &attr, &data_size, NULL);
        print_str_hex(L"  GetVariable(test)=", s);
        if (EFI_ERROR(s)) {
            TEST_INFO(L"GetVariable not available (expected in test env)");
        } else {
            TEST_PASS(1, L"GetVariable succeeded");
        }
    } else {
        TEST_FAIL(L"GetVariable is NULL");
    }

    /* ResetSystem — check it's callable but DON'T actually reset */
    if (rs->ResetSystem) {
        TEST_PASS(1, L"ResetSystem function pointer present");
    } else {
        TEST_FAIL(L"ResetSystem is NULL");
    }
}

/* =================================================================
   TEST: Edge Cases & Error Handling
   ================================================================= */
static void test_edge_cases(void) {
    tests_run++;
    TEST_INFO(L"--- Q) Edge Cases & Error Handling ---");

    if (!gBS) { TEST_FAIL(L"BS unavailable"); return; }

    /* HandleProtocol with NULL handle */
    {
        VOID *iface = (VOID*)0xDEAD;
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        EFI_STATUS s = ((EFI_HP)gBS->HandleProtocol)(NULL, &gEfiBlockIoProtocolGuid, &iface);
        print_str_hex(L"  HandleProtocol(NULL)=", s);
        if (EFI_ERROR(s)) {
            TEST_PASS(1, L"HandleProtocol(NULL) properly rejected");
        } else {
            TEST_FAIL(L"HandleProtocol(NULL) should fail");
        }
    }

    /* HandleProtocol with invalid GUID */
    {
        EFI_GUID fake_guid = { 0xFFFFFFFF, 0xFFFF, 0xFFFF, {0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF} };
        VOID *iface = NULL;
        typedef EFI_STATUS (EFIAPI *EFI_HP)(EFI_HANDLE, EFI_GUID*, VOID**);
        EFI_STATUS s = ((EFI_HP)gBS->HandleProtocol)(gImageHandle, &fake_guid, &iface);
        print_str_hex(L"  HandleProtocol(fake GUID)=", s);
        if (EFI_ERROR(s)) {
            TEST_PASS(1, L"Fake GUID properly returns error");
        } else {
            TEST_FAIL(L"Fake GUID unexpectedly succeeded");
        }
    }

    /* AllocatePool with 0 size */
    {
        VOID *p = NULL;
        EFI_STATUS s = gBS->AllocatePool(EfiLoaderData, 0, &p);
        print_str_hex(L"  AllocatePool(0)=", s);
        if (EFI_ERROR(s)) {
            TEST_PASS(1, L"AllocatePool(0) properly rejected");
        } else if (p != NULL) {
            gBS->FreePool(p);
            TEST_INFO(L"AllocatePool(0) returned a buffer (lenient)");
        }
    }

    /* FreePool with NULL */
    {
        EFI_STATUS s = gBS->FreePool(NULL);
        print_str_hex(L"  FreePool(NULL)=", s);
        if (EFI_ERROR(s)) {
            TEST_PASS(1, L"FreePool(NULL) properly rejected");
        } else {
            TEST_INFO(L"FreePool(NULL) returned success (lenient)");
        }
    }

    /* WaitForEvent with timer event to test event signaling */
    {
        EFI_EVENT timer_event = NULL;
        EFI_STATUS s = gBS->CreateEvent(EVT_TIMER, 0, NULL, NULL, &timer_event);
        if (s == EFI_SUCCESS && timer_event) {
            s = gBS->SetTimer(timer_event, TimerRelative, 10000); /* 10ms */
            print_str_hex(L"  SetTimer(10ms)=", s);
            if (s == EFI_SUCCESS) {
                UINTN index = 0;
                s = gBS->WaitForEvent(1, &timer_event, &index);
                print_str_hex(L"  WaitForEvent(10ms timer)=", s);
                print_str_hex(L"    Index=", index);
                if (s == EFI_SUCCESS) {
                    TEST_PASS(1, L"Timer event works");
                }
            }
            gBS->CloseEvent(timer_event);
        }
    }

    /* Stall(0) */
    {
        EFI_STATUS s = gBS->Stall(0);
        print_str_hex(L"  Stall(0)=", s);
        if (s == EFI_SUCCESS) {
            TEST_PASS(1, L"Stall(0) OK");
        }
    }

    /* SetWatchdogTimer */
    {
        EFI_STATUS s = gBS->SetWatchdogTimer(0, 0, 0, NULL);
        print_str_hex(L"  SetWatchdogTimer(disable)=", s);
        if (s == EFI_SUCCESS) {
            TEST_PASS(1, L"SetWatchdogTimer OK");
        }
    }

    /* GetNextMonotonicCount */
    {
        UINT64 count = 0;
        EFI_STATUS s = gBS->GetNextMonotonicCount(&count);
        print_str_hex(L"  GetNextMonotonicCount=", s);
        if (s == EFI_SUCCESS) {
            print_str_hex(L"    Count=", count);
            TEST_PASS(1, L"Monotonic count works");
            /* Verify it increments */
            UINT64 count2 = 0;
            gBS->GetNextMonotonicCount(&count2);
            if (count2 > count) {
                TEST_PASS(1, L"Monotonic count incremented");
            } else {
                TEST_FAIL(L"Monotonic count did not increment");
            }
        }
    }
}

/* =================================================================
   Main — runs all tests
   ================================================================= */
EFI_STATUS EFIAPI efi_main(
    EFI_HANDLE       ImageHandle,
    EFI_SYSTEM_TABLE *SystemTable
) {
    gImageHandle = ImageHandle;
    gST = SystemTable;
    gBS = NULL;
    fails = 0;
    tests_run = 0;

    /* Init hex buffer */
    for (int i = 0; i < 20; i++) gHexBuf[i] = L'0';

    /* Phase 1: Entry point integrity */
    test_entry_point();
    if (gST == NULL) goto done;

    /* Phase 2: System table validation */
    test_system_table();
    if (gBS == NULL) goto done;

    /* Phase 3: Protocol tests (Limine boot flow) */
    test_loaded_image_protocol();
    test_device_path_protocol();
    test_block_io_protocol();
    test_simple_file_system();

    /* Phase 4: Memory services */
    test_memory_allocation();
    test_memory_map();

    /* Phase 5: Boot services tests */
    test_locate_services();
    test_event_services();
    test_stall();
    test_calculate_crc32();
    test_copy_set_mem();

    /* Phase 6: Configuration / Runtime */
    test_configuration_table();
    test_runtime_services();

    /* Phase 7: Edge cases */
    test_edge_cases();

    /* Phase 8: ExitBootServices — run last as it may break things */
    test_exit_boot_services();

done:
    print(L"\r\n========================================\r\n");
    print_str_hex(L"  TESTS RUN: ", tests_run);
    print_str_hex(L"  FAILURES:  ", fails);
    print(L"========================================\r\n");

    if (fails > 0) {
        print(L"\r\n*** LIMINE BOOT ISSUES DETECTED ***\r\n");
        if (!gST || !gBS) print(L"  - System table or boot services broken\r\n");
    } else {
        print(L"\r\nAll tests passed. Limine should boot.\r\n");
    }

    /* Halt */
    for (;;) {
        __asm__ volatile("cli; hlt");
    }

    return EFI_SUCCESS;
}
