#include "boot.h"
#include "../headers/serial.h"
#include "../headers/errors.h"
#include "../headers/memcmp.h"
#include "../headers/sector_range.h"
#include "../disk/esp.h"
#include "../disk/fat32.h"

static int find_in_dir(Fat32_Handle* fs, DirEntry** out, const char* name, int len) {
    while (next_dir_entry(fs, out) == SUCCSESS) {
        if (memcmp((*out)->name, name, len) == 0)
            return SUCCSESS;
    }
    return NOT_A_DIRECTORY;
}

static int find_efi_bootx64(Fat32_Handle* fs, DirEntry** out) {
    DirEntry* entry;

    if (open_root_dir(fs) != SUCCSESS) return IO_ERROR;
    if (find_in_dir(fs, &entry, "EFI        ", 11) != SUCCSESS) {
        serial_puts("Did not find the EFI dir\n");
        return NOT_A_DIRECTORY;
    }
    if (open_dir_entry(fs, entry) != SUCCSESS) return IO_ERROR;
    if (find_in_dir(fs, &entry, "BOOT       ", 11) != SUCCSESS) {
        serial_puts("Did not find the BOOT dir\n");
        return NOT_A_DIRECTORY;
    }
    if (open_dir_entry(fs, entry) != SUCCSESS) return IO_ERROR;
    if (find_in_dir(fs, &entry, "BOOTX64 EFI", 11) != SUCCSESS) {
        serial_puts("Did not find the BOOTX64 binary\n");
        return NOT_A_DIRECTORY;
    }
    *out = entry;
    return SUCCSESS;
}

int load_efi_application(uint8_t* file_buf) {
    SectorRange sec_range;
    int status = load_part_table(&sec_range);
    if (status != 0) return status;

    static Fat32_Handle fs;
    status = open_fat32(&sec_range, &fs);
    if (status != 0) return status;

    DirEntry* entry;
    status = find_efi_bootx64(&fs, &entry);
    if (status != 0) return status;

    read_file(&fs, entry, file_buf, entry->file_size);

    return SUCCSESS;
}
