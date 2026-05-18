#include "headers/serial.h"
#include "virtio/blk.c"
#include "headers/idt.h"
#include "headers/sector_range.h"
#include "mem/memmap.c"
#include "mem/heap.c"
#include "disk/esp.c"
#include "disk/fat32.c"
#include "mem/stack.c"
#include "disk/format_PE.c"
#include "tss.c"
#include "headers/gdt.h"
#include "headers/halt.h"

void c_main_64(void) {
    uint64_t rsp;
    __asm__ volatile("mov %%rsp, %0" : "=r"(rsp));
    serial_puts("c_main_64 rsp=");
    serial_putx(rsp);
    serial_puts("\n");
    serial_puts("=-- Long mode --=\n");

    tss_init();
    gdt_set_tss(gdt64, 5);
    GDTPointer64 gdtp = {
        .size = sizeof(gdt64) - 1,
        .base = (uint64_t)gdt64
    };
    __asm__ volatile("lgdt %0" :: "m"(gdtp));
    tss_enable(5 * 8);
    
    idt_init();

    idt_init();
    init_memmap();
    init_heap(0x3000000, 0x4000000);
    serial_puts("heap after init=");
    serial_putx((uint64_t)heap_ptr);
    serial_puts("\n");
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
        if (memcmp(entry->name, "BOOTX64 EFI", 11) == 0){
            found_exe = 1;
            break;
        }
    }

    if (!found_exe) {
        serial_puts("Did not find the BOOTX64 binary");
        return;
    }

    uint8_t* file_buf = (uint8_t*)0x1000000;

    read_file(&fs, entry, file_buf, entry->file_size);
    for (int i = 0; i < 10; i++) {
        serial_putx(file_buf[i]);
    }
    serial_puts("\n");

    print_stack_usage();

    format_pe(file_buf);

    // spin forever
    hang();
}