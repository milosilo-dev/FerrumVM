#include "../headers/pe_exe.h"
#include "../headers/uefi/uefi.h"
#include "../headers/uefi/image_handle.h"
#include "../uefi.c"

// Base relocation block header
typedef struct {
    uint32_t VirtualAddress;
    uint32_t SizeOfBlock;
} BASE_RELOCATION_BLOCK;

#define IMAGE_REL_BASED_ABSOLUTE 0
#define IMAGE_REL_BASED_DIR64    10

static void apply_relocations(uint8_t* load_base, IMAGE_NT_HEADERS64* nt) {
    IMAGE_DATA_DIRECTORY* reloc_dir =
        &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_BASERELOC];

    // ---- VALIDATE DIRECTORY ----
    uint32_t dir_count = nt->OptionalHeader.NumberOfRvaAndSizes;
    if (IMAGE_DIRECTORY_ENTRY_BASERELOC >= dir_count) {
        serial_puts("pe_exe: no reloc dir\n");
        return;
    }

    if (reloc_dir->Size == 0 || reloc_dir->VirtualAddress == 0) {
        serial_puts("pe_exe: empty reloc dir\n");
        return;
    }

    int64_t delta = (int64_t)load_base - (int64_t)nt->OptionalHeader.ImageBase;
    if (delta == 0) return;

    uint8_t* reloc_data     = load_base + reloc_dir->VirtualAddress;
    uint8_t* reloc_data_end = reloc_data + reloc_dir->Size;

    while (reloc_data < reloc_data_end) {
        BASE_RELOCATION_BLOCK* block = (BASE_RELOCATION_BLOCK*)reloc_data;

        if (block->SizeOfBlock < sizeof(BASE_RELOCATION_BLOCK))
            break;

        uint32_t entry_count =
            (block->SizeOfBlock - sizeof(BASE_RELOCATION_BLOCK)) / 2;

        uint16_t* entries = (uint16_t*)(reloc_data + sizeof(BASE_RELOCATION_BLOCK));

        for (uint32_t i = 0; i < entry_count; i++) {
            uint8_t type = entries[i] >> 12;
            uint16_t offset = entries[i] & 0x0FFF;

            if (type == IMAGE_REL_BASED_ABSOLUTE)
                continue;

            if (type == IMAGE_REL_BASED_DIR64) {
                uint64_t* target =
                    (uint64_t*)(load_base + block->VirtualAddress + offset);
                *target += delta;
            }
        }

        reloc_data += block->SizeOfBlock;
    }
}

void format_pe(uint8_t* exe) {
    EFI_IMAGE_DOS_HEADER* dos = (EFI_IMAGE_DOS_HEADER*)exe;

    if (dos->e_magic != IMAGE_DOS_SIGNATURE) {
        serial_puts("pe_exe: bad DOS magic\n");
        return;
    }

    IMAGE_NT_HEADERS64* nt = (IMAGE_NT_HEADERS64*)(exe + dos->e_lfanew);

    if (nt->Signature != IMAGE_NT_SIGNATURE) {
        serial_puts("pe_exe: bad NT sig\n");
        return;
    }

    // ---- SAFE OPTIONAL HEADER ACCESS ----
    uint16_t opt_size = nt->FileHeader.SizeOfOptionalHeader;

    if (nt->OptionalHeader.Magic != 0x20B) {
        serial_puts("pe_exe: not PE32+\n");
        return;
    }

    if (nt->OptionalHeader.Subsystem != IMAGE_SUBSYSTEM_EFI_APPLICATION) {
        serial_puts("refusing non-EFI PE\n");
        return;
    }

    serial_puts("pe_exe: Sections = ");
    serial_putx(nt->FileHeader.NumberOfSections);
    serial_puts("\n");

    serial_puts("pe_exe: ImageBase = ");
    serial_putx(nt->OptionalHeader.ImageBase);
    serial_puts("\n");

    serial_puts("pe_exe: SizeOfImage = ");
    serial_putx(nt->OptionalHeader.SizeOfImage);
    serial_puts("\n");

    serial_puts("pe_exe: Entry RVA = ");
    serial_putx(nt->OptionalHeader.AddressOfEntryPoint);
    serial_puts("\n");

    uint8_t* load_base = (uint8_t*)0x1000000;

    memcpy(load_base, exe, nt->OptionalHeader.SizeOfHeaders);

    IMAGE_SECTION_HEADER* sections = (IMAGE_SECTION_HEADER*)(
        (uint8_t*)&nt->OptionalHeader + nt->FileHeader.SizeOfOptionalHeader
    );

    // ---- SAFE SECTION COPY ----
    for (uint16_t i = 0; i < nt->FileHeader.NumberOfSections; i++) {
        uint8_t* dst = load_base + sections[i].VirtualAddress;
        uint8_t* src = exe + sections[i].PointerToRawData;

        uint32_t raw_size  = sections[i].SizeOfRawData;
        uint32_t virt_size = sections[i].Misc.VirtualSize;

        if (raw_size > 0)
            memcpy(dst, src, raw_size);

        if (virt_size > raw_size)
            memset(dst + raw_size, 0, virt_size - raw_size);
    }

    // ---- SAFE RELOCATION ----
    apply_relocations(load_base, nt);

    // ---- BUILD FAKE UEFI ENV ----
    EFI_SYSTEM_TABLE* system_table = malloc(sizeof(EFI_SYSTEM_TABLE));
    memset(system_table, 0, sizeof(EFI_SYSTEM_TABLE));
    format_system_table(system_table);
    patch_null_stubs();

    EFI_IMAGE_HANDLE_DATA* handle_data = malloc(sizeof(EFI_IMAGE_HANDLE_DATA));
    memset(handle_data, 0, sizeof(EFI_IMAGE_HANDLE_DATA));
    format_handle_data(handle_data, system_table,
                        nt->OptionalHeader.SizeOfImage,
                        load_base);

    EFI_HANDLE image_handle = handle_data;

    uint64_t ep = (uint64_t)load_base + nt->OptionalHeader.AddressOfEntryPoint;

    serial_puts("pe_exe: jumping to = ");
    serial_putx(ep);
    serial_puts("\n");

    // ---- DEBUG SAFE DIRECTORY DUMP ----
    uint32_t dir_count = nt->OptionalHeader.NumberOfRvaAndSizes;
    if (dir_count > IMAGE_NUMBEROF_DIRECTORY_ENTRIES)
        dir_count = IMAGE_NUMBEROF_DIRECTORY_ENTRIES;

    for (uint32_t i = 0; i < dir_count; i++) {
        IMAGE_DATA_DIRECTORY* dir =
            get_dir(&nt->OptionalHeader, i, nt->FileHeader.SizeOfOptionalHeader);

        if (!dir) continue;

        serial_puts("dir ");
        serial_putx(i);
        serial_puts(" va=");
        serial_putx(dir->VirtualAddress);
        serial_puts(" size=");
        serial_putx(dir->Size);
        serial_puts("\n");
    }

    serial_puts("pe_exe: image handle addr = ");
    serial_putx((uint64_t)image_handle);
    serial_puts("\n");

    serial_puts("pe_exe: system table addr = ");
    serial_putx((uint64_t)system_table);
    serial_puts("\n");

    serial_puts("BootServices addr=");
    serial_putx((uint64_t)&gBootServices);
    serial_puts("\n");
    serial_puts("AllocatePool ptr=");
    serial_putx((uint64_t)gBootServices.AllocatePool);
    serial_puts("\n");
    serial_puts("BootServices CRC=");
    serial_putx(gBootServices.Hdr.CRC32);
    serial_puts("\n");

    serial_putx(offsetof(EFI_BOOT_SERVICES, RaiseTPL));
    serial_putx(offsetof(EFI_BOOT_SERVICES, AllocatePages));
    serial_putx(offsetof(EFI_BOOT_SERVICES, AllocatePool));
    serial_putx(offsetof(EFI_BOOT_SERVICES, GetMemoryMap));
    serial_putx(offsetof(EFI_BOOT_SERVICES, ExitBootServices));

    // ---- CALL ENTRY ----
    EFI_STATUS status;

    __asm__ volatile (
        "mov %%rsp, %%r11\n"
        "and $-16, %%rsp\n"
        "sub $32, %%rsp\n"

        "mov %1, %%rcx\n"
        "mov %2, %%rdx\n"
        "call *%3\n"

        "mov %%rax, %0\n"

        "mov %%r11, %%rsp\n"

        : "=r"(status)
        : "r"((uint64_t)image_handle),
          "r"((uint64_t)system_table),
          "r"(ep)
        : "rax","rcx","rdx","r11","memory"
    );

    serial_puts("pe_exe: entry returned = ");
    serial_putx(status);
    serial_puts("\n");
}