#include "headers/serial.h"
#include "virtio/blk.c"
#include "headers/idt.h"
#include "headers/sector_range.h"
#include "disk/esp.c"
#include "disk/fat32.c"
#include "memmap.c"

void c_main_64(void) {
    serial_puts("=-- Long mode --=\n");
    idt_init();
    init_memmap();
    virtio_blk_init();

    SectorRange sec_range;
    int status = load_part_table(&sec_range);
    if (status != 0) return;

    Fat32_Handle fs;
    status = open_fat32(&sec_range, &fs);
    if (status != 0) return;

    status = open_root_dir(&fs);
    if (status != 0) return;

    DirEntry* entry;
    int found_efi;
    while (next_dir_entry(&fs, &entry) == SUCCSESS) {
        if (memcmp(entry->name, "EFI        ", 11) == 0){
            found_efi = 1;
            break;
        }
    }

    if (!found_efi) {
        serial_puts("Did not find the EFI dir");
        return;
    }
    found_efi = 0;

    // spin forever
    while (1) {
        __asm__ volatile("hlt");
    }
}