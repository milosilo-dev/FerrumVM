#include "format_PE.h"
#include "../headers/pe_exe.h"
#include "../efi/uefi.h"
#include "../mem/heap.h"

// Base relocation block header
typedef struct {
    uint32_t VirtualAddress;
    uint32_t SizeOfBlock;
} BASE_RELOCATION_BLOCK;

#define IMAGE_REL_BASED_ABSOLUTE 0
#define IMAGE_REL_BASED_DIR64    10

static void dump_ptr(const char* name, void* ptr) {
    serial_puts(name);
    serial_puts(" = 0x");
    serial_putx((uint64_t)ptr);
    serial_puts("\n");
}

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

        uint32_t reloc_count = 0;
        for (uint32_t i = 0; i < entry_count; i++) {
            uint8_t type = entries[i] >> 12;
            uint16_t offset = entries[i] & 0x0FFF;

            if (type == IMAGE_REL_BASED_ABSOLUTE)
                continue;

            if (type == IMAGE_REL_BASED_DIR64) {
                uint64_t* target =
                    (uint64_t*)(load_base + block->VirtualAddress + offset);
                *target += delta;
                reloc_count++;
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

    uint8_t* load_base = (uint8_t*)0x1200000;

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

    // Patch ImageBase in BOTH the loaded copy AND the raw file buffer,
    // because the shell may read from either location.
    uint64_t orig_image_base = nt->OptionalHeader.ImageBase;
    *((uint64_t*)(load_base + ((uint8_t*)&nt->OptionalHeader.ImageBase - exe))) = (uint64_t)load_base;
    nt->OptionalHeader.ImageBase = (uint64_t)load_base;

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

    // make the real LoadedImageProtocol available to HandleProtocol/OpenProtocol
    gLoadedImageInstance = &handle_data->loaded_image;

    efi_init(system_table, image_handle);

    // ---- DEBUG SAFE DIRECTORY DUMP ----
    uint32_t dir_count = nt->OptionalHeader.NumberOfRvaAndSizes;
    if (dir_count > IMAGE_NUMBEROF_DIRECTORY_ENTRIES)
        dir_count = IMAGE_NUMBEROF_DIRECTORY_ENTRIES;

    uint64_t rsp;
    __asm__ volatile("mov %%rsp, %0" : "=r"(rsp));
    uint64_t stack_bottom = rsp - 0x100000;

    if ((uint64_t)heap_ptr >= stack_bottom && (uint64_t)heap_ptr <= rsp) {
        serial_puts("ERROR: heap overlaps stack!\n");
    } else if ((uint64_t)load_base + nt->OptionalHeader.SizeOfImage >= (uint64_t)heap_ptr && 
             (uint64_t)load_base < (uint64_t)heap_end) {
        serial_puts("ERROR: heap overlaps PE!\n");
    }

    // ---- CALL ENTRY ----
    EFI_STATUS status;
    uint64_t saved_rsp;

    serial_puts("------------------ { Firmware init --> image } ------------------\n");

    __asm__ volatile (
        "mov %%rsp, %[sr]\n"
        "and $-16, %%rsp\n"
        "sub $32, %%rsp\n"
        "mov %[h], %%rcx\n"
        "mov %[st], %%rdx\n"
        "call *%[ep]\n"
        "mov %%rax, %[st2]\n"
        "mov %[sr], %%rsp\n"
        : [sr] "=&r"(saved_rsp),
          [st2] "=a"(status)
        : [h] "r"((uint64_t)image_handle),
          [st] "r"((uint64_t)system_table),
          [ep] "r"(ep)
        : "rcx","rdx","memory"
    );

    serial_puts("pe_exe: entry returned = ");
    serial_putx(status);
    serial_puts("\n");
}