#include <stdint.h>
#include "../headers/memcmp.h"
#include "../headers/gpt.h"

#define PART_LOAD_SUCCSESS 0;
#define INVALID_BOOT_SIGNITURE 1;
#define CORRUPT_GPT 2;
#define CORRUPT_MBR 3;
#define IO_ERROR 4;
#define NOT_GPT 5;
#define INVALID_REVSION 6;

typedef struct {
    uint64_t first_sector;
    uint64_t last_sector;
} __attribute__((packed)) SectorRange;

int load_part_table(SectorRange* range) {
    uint8_t sector[512];
    uint32_t status = virtio_blk_read(0, 512, sector);
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

    uint8_t* part_type = (void*)(sector + 0x1BE + 4);

    if (*part_type != 0xEE)  {
        serial_puts("part-table: MBR Detected\n");
        return NOT_GPT;
    }

    uint8_t sector2[512];
    status = virtio_blk_read(1, 512, sector2);
    if (status != 0) {
        serial_puts("part-table: GPT read failed\n");
        return IO_ERROR;
    }

    GPTHeader* gpt_header = (GPTHeader*)&sector2;

    if (memcmp(gpt_header->signature, "EFI PART", 8) != 0) {
        serial_puts("part-table: Corrupt GPT");
        return CORRUPT_GPT;
    }

    if (gpt_header->revision == 0x0001000) {
        serial_puts("part-table: Incompatible revision");
        return INVALID_REVSION;
    }
    serial_puts("part-table: GPT Detected\n");

    uint64_t table_lba = gpt_header->partition_entries_lba;
    uint32_t entry_size = gpt_header->partition_entry_size;
    uint32_t entry_count = gpt_header->num_partition_entries;

    uint64_t table_bytes = entry_count * entry_size;
    uint64_t sectors = (table_bytes + 511) / 512;

    uint8_t part_entries_buf[sectors * 512];
    status = virtio_blk_read(table_lba, sectors * 512 / 512, part_entries_buf);

    for (uint32_t i = 0; i < entry_count; i++) {

        GPTPartitionEntry *entry =
            (GPTPartitionEntry*)(part_entries_buf + i * entry_size);

        // skip empty entry
        int empty = 1;
        for (int j = 0; j < 16; j++) {
            if (entry->type_guid[j] != 0) {
                empty = 0;
                break;
            }
        }

        if (empty)
            continue;

        serial_puts("part-table: Entry ");
        serial_putx(i);
        serial_puts(" valid\n");
    }

    return PART_LOAD_SUCCSESS;
}