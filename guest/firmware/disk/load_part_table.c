#include <stdint.h>
#include "../headers/memcmp.h"
#include "../headers/mbr.h"

#define PART_LOAD_SUCCSESS 0;
#define INVALID_BOOT_SIGNITURE 1;
#define CORRUPT_GPT 2;
#define CORRUPT_MBR 3;
#define IO_ERROR 4;

int load_part_table() {
    uint8_t sector[512];
    uint32_t status = virtio_blk_read(0, 512, sector);
    serial_puts("part-table: blk read status = "); serial_putx(status); serial_puts("\n");
    if (status != 0) {
        serial_puts("part-table: GPT read failed\n");
        return IO_ERROR;
    }

    if (*(uint16_t*)(sector + 0x1FE) != 0xAA55){
        serial_puts("part-table: Invalid boot signiture");
        serial_puts("part-table: Boot sig = ");
        serial_putx(sector[510]); serial_putc(' ');
        serial_putx(sector[511]); serial_puts("\n");
        return INVALID_BOOT_SIGNITURE;
    }

    MBRPartition* partions = (void*)(sector + 0x1BE);
    for (int i = 0; i < 4; i++) {
        serial_puts("Partition ");
        serial_putx(i);
        serial_puts(": type=");
        serial_putx(partions[i].type);
        serial_puts(" start=");
        serial_putx(partions[i].lba_start);
        serial_puts(" sectors=");
        serial_putx(partions[i].sectors);
        serial_puts(" status=");
        serial_putx(partions[i].status);
        serial_puts("\n");
    }

    if (partions->type == 0xEE) {
        // GPT
        uint8_t sector2[512];
        uint32_t status = virtio_blk_read(1, 512, sector2);
        serial_puts("part-table: blk read status = "); serial_putx(status); serial_puts("\n");
        if (status != 0) {
            serial_puts("part-table: GPT read failed\n");
            return IO_ERROR;
        }

        if (memcmp(sector2, "EFI PART", 8) != 0) {
            serial_puts("part-table: Corrupt GPT");
            return CORRUPT_GPT;
        }
        serial_puts("part-table: GPT Detected");
        // TODO: Finsh GPT path
    } else {
        serial_puts("part-table: MBR Detected\n");
    }

    return PART_LOAD_SUCCSESS;
}