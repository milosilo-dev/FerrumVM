#pragma once
#include <stdint.h>
#include "../headers/sector_range.h"
#include "../headers/errors.h"

typedef struct {
    char name[11];
    uint8_t attr;
    uint8_t reserved;
    uint8_t creation_time_tenth;
    uint16_t creation_time;
    uint16_t creation_date;
    uint16_t last_access_date;
    uint16_t first_cluster_high;
    uint16_t write_time;
    uint16_t write_date;
    uint16_t first_cluster_low;
    uint32_t file_size;
} __attribute__((packed)) DirEntry;

typedef struct {
    uint32_t length;
    DirEntry* first_entry;
} __attribute__((packed)) DirList;

typedef struct {
    uint16_t bytes_per_sector;
    uint8_t  sectors_per_cluster;
    uint16_t reserved_sectors;
    uint8_t  num_fats;
    uint32_t fat_size;
    uint32_t root_cluster;
} FAT32_BPB;

typedef struct {
    FAT32_BPB fat;
    SectorRange* range;
    uint32_t data_start_lba;
    uint32_t cluster;
    uint32_t cluster_size;
    uint8_t  cluster_buf[32768];
    uint32_t current_cluster;
    uint32_t entry_index;
} __attribute__((packed)) Fat32_Handle;

int open_fat32(SectorRange* range, Fat32_Handle* fs);
int open_root_dir(Fat32_Handle* fs);
int next_dir_entry(Fat32_Handle* fs, DirEntry** out_entry);
int open_dir_entry(Fat32_Handle* fs, DirEntry* entry);
int read_file(Fat32_Handle* fs, DirEntry* entry, uint8_t* buf, uint32_t buf_size);
int read_file_offset(Fat32_Handle* fs, DirEntry* entry, uint8_t* buf, uint32_t buf_size, uint32_t read_offset);
uint32_t cluster_to_lba(Fat32_Handle* fs, uint32_t cluster);
int read_fat_entry(Fat32_Handle* fs, uint32_t cluster, uint32_t* next_cluster);
