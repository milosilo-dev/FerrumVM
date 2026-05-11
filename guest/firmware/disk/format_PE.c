#include "../headers/pe_exe.h"
#include "../headers/uefi/uefi.h"
#include "../headers/uefi/image_handle.h"
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

    uint64_t image_base = (uint64_t)nt_header->OptionalHeader.ImageBase;
    EFI_IMAGE_ENTRY_POINT entry = (EFI_IMAGE_ENTRY_POINT)(image_base + ep_rva);

    EFI_SYSTEM_TABLE* system_table = (EFI_SYSTEM_TABLE*)malloc(sizeof(EFI_SYSTEM_TABLE));
    format_system_table(system_table);

    FAKE_IMAGE_HANDLE_DATA* handle_data = malloc(sizeof(FAKE_IMAGE_HANDLE_DATA));

    // Fill in loaded image protocol fields
    handle_data->loaded_image.ImageBase = exe;
    handle_data->loaded_image.ImageSize = nt_header->OptionalHeader.SizeOfImage;
    handle_data->loaded_image.SystemTable = system_table;

    EFI_HANDLE image_handle = handle_data;

    entry(image_handle, system_table);
}