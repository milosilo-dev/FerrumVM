#include <stdint.h>
#include <uchar.h>

typedef struct {
    char     signature[8];        // "EFI PART"
    uint32_t revision;            // 0x00010000
    uint32_t header_size;         // usually 92
    uint32_t header_crc32;        // CRC of header (with this field zeroed)
    uint32_t reserved;            // must be 0

    uint64_t current_lba;         // usually 1
    uint64_t backup_lba;          // last sector of disk
    uint64_t first_usable_lba;
    uint64_t last_usable_lba;

    uint8_t  disk_guid[16];       // unique disk ID

    uint64_t partition_entries_lba;
    uint32_t num_partition_entries;
    uint32_t partition_entry_size; // usually 128

    uint32_t partition_array_crc32;
} __attribute__((packed)) GPTHeader;

typedef struct {
    uint8_t  type_guid[16];
    uint8_t  unique_guid[16];
    uint64_t first_lba;
    uint64_t last_lba;
    uint64_t attributes;
    char16_t name[36];
} __attribute__((packed)) GPTPartitionEntry;