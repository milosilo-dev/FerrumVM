#include "../headers/pe_exe.h"

void format_pe(uint8_t* exe) {
    EFI_IMAGE_DOS_HEADER* dos_header = (EFI_IMAGE_DOS_HEADER*)exe;
    IMAGE_NT_HEADERS64* nt_header = (IMAGE_NT_HEADERS64*)(dos_header->e_lfanew + (uint64_t)exe);

    serial_puts("pe_exe: Dos Magic value = ");
    serial_putx(dos_header->e_magic);
    serial_puts("\n");

    serial_puts("pe_exe: NT Signiture = ");
    serial_putx(nt_header->Signature);
    serial_puts("\n");
}