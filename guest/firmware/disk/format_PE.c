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

        serial_puts("relocations applied=");
        serial_putx(reloc_count);
        serial_puts("\n");

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
    serial_puts("pe_exe: patched ImageBase to ");
    serial_putx((uint64_t)load_base);
    serial_puts(" (was ");
    serial_putx(orig_image_base);
    serial_puts(")\n");

    // The shell reads function pointers from firmware tables that have
    // stale entries pointing to data instead of valid code. Patch the
    // entire likely table range in firmware with a small RET-plus-NOP
    // stub so any entry the shell tries to call will just return.
    // The firmware data area (0x106xxx-0x107xxx) contains static
    // UEFI protocol structure instances whose trailing padding can be
    // misread by the shell as function-pointer tables.
    serial_puts("pe_exe: patching firmware data pointers to RET stubs\n");
    uint8_t* fw_data_start = (uint8_t*)0x106000;
    uint8_t* fw_data_end   = (uint8_t*)0x107F00;
    // walk 8-byte aligned and replace any value that looks like it
    // points into the firmware's own data range (0x106xxx-0x107Exx)
    // with a RET-stub address.
    uint32_t patched = 0;
    for (uint8_t* p = fw_data_start; p < fw_data_end; p += 8) {
        uint64_t val = *(uint64_t*)p;
        if (val >= 0x106000 && val < 0x108000) {
            // this pointer targets firmware data — replace target byte with RET
            uint8_t* tgt = (uint8_t*)val;
            if (*tgt != 0xC3) {
                *tgt = 0xC3;
                patched++;
            }
        }
    }
    serial_puts("pe_exe: patched ");
    serial_putx(patched);
    serial_puts(" firmware data pointers -> RET\n");

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

    serial_puts("pe_exe: jumping to = ");
    serial_putx(ep);
    serial_puts("\n");

    // ---- DEBUG SAFE DIRECTORY DUMP ----
    uint32_t dir_count = nt->OptionalHeader.NumberOfRvaAndSizes;
    if (dir_count > IMAGE_NUMBEROF_DIRECTORY_ENTRIES)
        dir_count = IMAGE_NUMBEROF_DIRECTORY_ENTRIES;

    serial_puts("=== MEMORY LAYOUT ===\n");

    uint64_t rsp;
    __asm__ volatile("mov %%rsp, %0" : "=r"(rsp));
    serial_puts("stack ptr=");
    serial_putx(rsp);
    serial_puts("\n");

    serial_puts("heap start=");
    serial_putx((uint64_t)heap_ptr);
    serial_puts("\n");

    serial_puts("heap end=");
    serial_putx((uint64_t)heap_end);
    serial_puts("\n");

    serial_puts("PE load base=");
    serial_putx((uint64_t)load_base);
    serial_puts("\n");
    serial_puts("PE end=");
    serial_putx((uint64_t)load_base + nt->OptionalHeader.SizeOfImage);
    serial_puts("\n");

    // check overlap
    uint64_t stack_bottom = rsp - 0x100000; // assume 1MB stack usage max
    serial_puts("assumed stack bottom=");
    serial_putx(stack_bottom);
    serial_puts("\n");

    if ((uint64_t)heap_ptr >= stack_bottom && (uint64_t)heap_ptr <= rsp) {
        serial_puts("ERROR: heap overlaps stack!\n");
    } else if ((uint64_t)load_base + nt->OptionalHeader.SizeOfImage >= (uint64_t)heap_ptr && 
             (uint64_t)load_base < (uint64_t)heap_end) {
        serial_puts("ERROR: heap overlaps PE!\n");
    } else {
        serial_puts("OK: no overlap detected\n");
    }

    serial_puts("=== END LAYOUT ===\n");
    serial_puts("heap before PE=");
    serial_putx((uint64_t)heap_ptr);
    serial_puts("\n");

    // ---- CALL ENTRY ----
    EFI_STATUS status;
    uint64_t saved_rsp;

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