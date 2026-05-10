#include "../headers/pe_exe.h"
#include "../headers/uefi/uefi.h"
#include "../uefi.c"

void format_pe(uint8_t* exe) {
    EFI_IMAGE_DOS_HEADER* dos_header = (EFI_IMAGE_DOS_HEADER*)exe;
    IMAGE_NT_HEADERS64* nt_header = (IMAGE_NT_HEADERS64*)(dos_header->e_lfanew + (uint64_t)exe);

    serial_puts("pe_exe: Dos Magic value = ");
    serial_putx(dos_header->e_magic);
    serial_puts("\n");

    serial_puts("pe_exe: NT Signiture = ");
    serial_putx(nt_header->Signature);
    serial_puts("\n");

    serial_puts("pe_exe: NT Magic number = ");
    serial_putx(nt_header->OptionalHeader.Magic);
    serial_puts("\n");

    uint32_t ep_rva = nt_header->OptionalHeader.AddressOfEntryPoint;

    serial_puts("Entry RVA: ");
    serial_putx(ep_rva);
    serial_puts("\n");

    EFI_IMAGE_ENTRY_POINT entry = (EFI_IMAGE_ENTRY_POINT)(
        (uint8_t *)exe + ep_rva
    );

    EFI_HANDLE image_handle = exe;
    EFI_SYSTEM_TABLE* system_table = (EFI_SYSTEM_TABLE*)malloc(sizeof(EFI_SYSTEM_TABLE));
    format_system_table(system_table);

    entry(image_handle, system_table);
}