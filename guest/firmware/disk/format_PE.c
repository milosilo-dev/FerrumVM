#include "../headers/pe_exe.h"
#include "../headers/uefi/uefi.h"
#include "../headers/uefi/image_handle.h"
#include "../uefi.c"

// Base relocation block header
typedef struct {
    uint32_t VirtualAddress;
    uint32_t SizeOfBlock;
} BASE_RELOCATION_BLOCK;

// Each relocation entry is a 16-bit value:
// top 4 bits = type, bottom 12 bits = offset within page
#define IMAGE_REL_BASED_ABSOLUTE 0
#define IMAGE_REL_BASED_DIR64    10

static void apply_relocations(uint8_t* load_base, IMAGE_NT_HEADERS64* nt) {
    IMAGE_DATA_DIRECTORY* reloc_dir =
        &nt->OptionalHeader.DataDirectory[IMAGE_DIRECTORY_ENTRY_BASERELOC];

    if (reloc_dir->Size == 0) {
        serial_puts("pe_exe: no relocation table, assuming load at ImageBase\n");
        return;
    }

    int64_t delta = (int64_t)load_base - (int64_t)nt->OptionalHeader.ImageBase;
    if (delta == 0) return;  // loaded at preferred base, nothing to fix up

    uint8_t* reloc_data     = load_base + reloc_dir->VirtualAddress;
    uint8_t* reloc_data_end = reloc_data + reloc_dir->Size;

    while (reloc_data < reloc_data_end) {
        BASE_RELOCATION_BLOCK* block = (BASE_RELOCATION_BLOCK*)reloc_data;
        if (block->SizeOfBlock == 0) break;

        uint32_t entry_count = (block->SizeOfBlock - sizeof(BASE_RELOCATION_BLOCK)) / 2;
        uint16_t* entries    = (uint16_t*)(reloc_data + sizeof(BASE_RELOCATION_BLOCK));

        for (uint32_t i = 0; i < entry_count; i++) {
            uint8_t  type   = entries[i] >> 12;
            uint16_t offset = entries[i] & 0x0FFF;

            if (type == IMAGE_REL_BASED_ABSOLUTE) continue;  // padding, skip

            if (type == IMAGE_REL_BASED_DIR64) {
                uint64_t* target = (uint64_t*)(load_base + block->VirtualAddress + offset);
                *target += delta;
            } else {
                serial_puts("pe_exe: unknown relocation type, skipping\n");
            }
        }

        reloc_data += block->SizeOfBlock;
    }
}

void format_pe(uint8_t* exe) {
    EFI_IMAGE_DOS_HEADER* dos = (EFI_IMAGE_DOS_HEADER*)exe;
    if (dos->e_magic != 0x5A4D) { serial_puts("pe_exe: bad DOS magic\n"); return; }

    IMAGE_NT_HEADERS64* nt = (IMAGE_NT_HEADERS64*)(exe + dos->e_lfanew);
    if (nt->Signature != 0x00004550)  { serial_puts("pe_exe: bad NT sig\n");  return; }
    if (nt->OptionalHeader.Magic != 0x20B) { serial_puts("pe_exe: not PE32+\n"); return; }

    serial_puts("pe_exe: Dos Magic     = "); serial_putx(dos->e_magic);                       serial_puts("\n");
    serial_puts("pe_exe: NT Signature  = "); serial_putx(nt->Signature);                      serial_puts("\n");
    serial_puts("pe_exe: NT Magic      = "); serial_putx(nt->OptionalHeader.Magic);            serial_puts("\n");
    serial_puts("pe_exe: Sections      = "); serial_putx(nt->FileHeader.NumberOfSections);     serial_puts("\n");
    serial_puts("pe_exe: ImageBase     = "); serial_putx(nt->OptionalHeader.ImageBase);        serial_puts("\n");
    serial_puts("pe_exe: SizeOfImage   = "); serial_putx(nt->OptionalHeader.SizeOfImage);      serial_puts("\n");
    serial_puts("pe_exe: Entry RVA     = "); serial_putx(nt->OptionalHeader.AddressOfEntryPoint); serial_puts("\n");

    // Pick a safe load address — well away from heap (0x400000-0x800000)
    // and stack (top of low memory). 1MB aligned, plenty of room.
    uint8_t* load_base = (uint8_t*)0x1000000;  // 16MB mark

    serial_puts("pe_exe: load base     = "); serial_putx((uint64_t)load_base); serial_puts("\n");

    // Copy headers
    memcpy(load_base, exe, nt->OptionalHeader.SizeOfHeaders);

    // Copy sections
    IMAGE_SECTION_HEADER* sections = (IMAGE_SECTION_HEADER*)(
        (uint8_t*)&nt->OptionalHeader + nt->FileHeader.SizeOfOptionalHeader
    );

    for (uint16_t i = 0; i < nt->FileHeader.NumberOfSections; i++) {
        uint8_t* dst       = load_base + sections[i].VirtualAddress;
        uint8_t* src       = exe       + sections[i].PointerToRawData;
        uint32_t raw_size  = sections[i].SizeOfRawData;
        uint32_t virt_size = sections[i].Misc.VirtualSize;

        serial_puts("pe_exe: section rva="); serial_putx(sections[i].VirtualAddress);
        serial_puts(" raw=");               serial_putx(raw_size);
        serial_puts(" virt=");              serial_putx(virt_size);
        serial_puts("\n");

        if (raw_size > 0)
            memcpy(dst, src, raw_size);
        if (virt_size > raw_size)
            memset(dst + raw_size, 0, virt_size - raw_size);
    }

    // Apply relocations — delta is load_base minus preferred base
    // Since ImageBase is 0, delta == load_base exactly
    apply_relocations(load_base, nt);

    // Build fake UEFI environment
    EFI_SYSTEM_TABLE* system_table = malloc(sizeof(EFI_SYSTEM_TABLE));
    memset(system_table, 0, sizeof(EFI_SYSTEM_TABLE));
    format_system_table(system_table);

    EFI_IMAGE_HANDLE_DATA* handle_data = malloc(sizeof(EFI_IMAGE_HANDLE_DATA));
    memset(handle_data, 0, sizeof(EFI_IMAGE_HANDLE_DATA));
    format_handle_data(handle_data, system_table, nt->OptionalHeader.SizeOfImage, load_base);
    EFI_HANDLE image_handle = handle_data;

    uint64_t ep = (uint64_t)load_base + nt->OptionalHeader.AddressOfEntryPoint;
    serial_puts("pe_exe: jumping to    = "); serial_putx(ep); serial_puts("\n");

    serial_puts("pe_exe: VirtualAddress=");
    serial_putx(nt->OptionalHeader.DataDirectory[5].VirtualAddress);
    serial_puts("\n");
    serial_puts("pe_exe: Size=");
    serial_putx(nt->OptionalHeader.DataDirectory[5].Size);
    serial_puts("\n");

    EFI_STATUS status;
    __asm__ volatile (
        "mov %%rsp, %%r11       \n"   // save original rsp
        "sub $32, %%rsp         \n"   // shadow space

        "mov %1, %%rcx          \n"   // arg1
        "mov %2, %%rdx          \n"   // arg2

        "call *%3               \n"

        "mov %%rax, %0          \n"

        "mov %%r11, %%rsp       \n"   // restore ORIGINAL rsp
        : "=r"(status)
        : "r"((uint64_t)image_handle),
        "r"((uint64_t)system_table),
        "r"(ep)
        : "rax", "rcx", "rdx",
        "r8", "r9", "r10", "r11",
        "memory"
    );
    serial_puts("pe_exe: entry returned = "); serial_putx(status); serial_puts("\n");
}