#include "headers/serial.h"
#include "virtio/blk.c"
#include "headers/idt.h"
#include "headers/sector_range.h"
#include "disk/esp.c"
#include "disk/fat32.c"
#include "disk/format_PE.c"
#include "mem/stack.c"
#include "mem/memmap.c"
#include "mem/heap.c"

void c_main_64(void) {
    serial_puts("=-- Long mode --=\n");
    idt_init();
    init_memmap();
    init_heap(0x00200000, 0x00300000);
    virtio_blk_init();

    SectorRange sec_range;
    int status = load_part_table(&sec_range);
    if (status != 0) return;

    static Fat32_Handle fs;
    status = open_fat32(&sec_range, &fs);
    if (status != 0) return;

    status = open_root_dir(&fs);
    if (status != 0) return;

    DirEntry* entry;
    int found_efi = 0;
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

    open_dir_entry(&fs, entry);
    int found_boot = 0;
    while (next_dir_entry(&fs, &entry) == SUCCSESS) {
        if (memcmp(entry->name, "BOOT       ", 11) == 0){
            found_boot = 1;
            break;
        }
    }

    if (!found_boot) {
        serial_puts("Did not find the Boot dir");
        return;
    }

    open_dir_entry(&fs, entry);
    int found_exe = 0;
    while (next_dir_entry(&fs, &entry) == SUCCSESS) {
        if (memcmp(entry->name, "BOOTX64 EFI ", 11) == 0){
            found_exe = 1;
            break;
        }
    }

    if (!found_exe) {
        serial_puts("Did not find the BOOTX64 binary");
        return;
    }

    uint8_t* file_buf = malloc(entry->file_size);

    read_file(&fs, entry, file_buf, sizeof(file_buf));
    for (int i = 0; i < 10; i++) {
        serial_putx(file_buf[i]);
    }
    serial_puts("\n");

    print_stack_usage();

    format_pe(file_buf);

    // spin forever
    while (1) {
        __asm__ volatile("hlt");
    }
}