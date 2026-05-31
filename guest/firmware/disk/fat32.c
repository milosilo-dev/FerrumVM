#include "fat32.h"
#include "../headers/serial.h"

uint32_t cluster_to_lba(Fat32_Handle* fs, uint32_t cluster) {
    return fs->data_start_lba +
           (cluster - 2) * fs->fat.sectors_per_cluster;
}

int open_fat32(SectorRange* range, Fat32_Handle* fs) {
    uint8_t bpb[512];
    int status = virtio_blk_read(range->first_sector + 0, 512, bpb);
    if (status != 0) {
        serial_puts("fat32: could not read BPB\n");
        return IO_ERROR;
    }

    FAT32_BPB fat;
    fat.bytes_per_sector   = *(uint16_t*)(bpb + 11);
    fat.sectors_per_cluster= *(uint8_t*)(bpb + 13);
    fat.reserved_sectors   = *(uint16_t*)(bpb + 14);
    fat.num_fats           = *(uint8_t*)(bpb + 16);
    fat.fat_size           = *(uint32_t*)(bpb + 36);
    fat.root_cluster       = *(uint32_t*)(bpb + 44);

    if (fat.bytes_per_sector != 512) {
        serial_puts("fat32: Invalid sector size of ");
        serial_putx(fat.bytes_per_sector);
        serial_puts("\n");
        return INVALID_FAT_SECTOR_SIZE;
    }

    if (fat.sectors_per_cluster == 0) {
        serial_puts("fat32: Invalid cluster size of 0\n");
        return INVALID_FAT_CLUSTER_SIZE;
    }

    uint32_t data_start_lba = fat.reserved_sectors + fat.num_fats * fat.fat_size;
    uint32_t cluster = fat.root_cluster;
    uint32_t cluster_size = fat.sectors_per_cluster * fat.bytes_per_sector;

    fs->cluster = cluster;
    fs->cluster_size = cluster_size;
    fs->data_start_lba = data_start_lba;
    fs->fat = fat;
    fs->range = range;

    return SUCCSESS;
}

static int read_fat_entry(Fat32_Handle* fs, uint32_t cluster, uint32_t* next_cluster) {
    uint32_t fat_offset    = cluster * 4;
    uint32_t fat_sector    = fs->fat.reserved_sectors + (fat_offset / fs->fat.bytes_per_sector);
    uint32_t entry_offset  = fat_offset % fs->fat.bytes_per_sector;

    uint8_t sector_buf[512];
    int status = virtio_blk_read(fs->range->first_sector + fat_sector, 512, sector_buf);
    if (status != 0) {
        serial_puts("fat32: failed to read FAT sector\n");
        return IO_ERROR;
    }

    uint32_t val = *(uint32_t*)(sector_buf + entry_offset) & 0x0FFFFFFF;
    *next_cluster = val;
    return SUCCSESS;
}

static int load_cluster(Fat32_Handle* fs, uint32_t cluster) {
    uint32_t lba = cluster_to_lba(fs, cluster);
    int status = virtio_blk_read(
        fs->range->first_sector + lba,
        fs->fat.sectors_per_cluster * fs->fat.bytes_per_sector,
        fs->cluster_buf
    );
    if (status != 0) {
        serial_puts("fat32: failed to read cluster\n");
        return IO_ERROR;
    }
    fs->current_cluster = cluster;
    fs->entry_index     = 0;
    return SUCCSESS;
}

int open_root_dir(Fat32_Handle* fs) {
    return load_cluster(fs, fs->fat.root_cluster);
}

int open_dir_entry(Fat32_Handle* fs, DirEntry* entry) {
    uint32_t cluster = ((uint32_t)entry->first_cluster_high << 16)
                      | entry->first_cluster_low;

    if (cluster == 0) {
        cluster = fs->fat.root_cluster;
    }

    if (!(entry->attr & 0x10)) {
        serial_puts("fat32: entry is not a directory\n");
        return NOT_A_DIRECTORY;
    }

    return load_cluster(fs, cluster);
}

int read_file_offset(Fat32_Handle* fs, DirEntry* entry, uint8_t* buf, uint32_t buf_size, uint32_t read_offset) {
        if (entry->attr & 0x10) {
        serial_puts("fat32: entry is a directory, not a file\n");
        return NOT_A_FILE;
    }

    uint32_t cluster = ((uint32_t)entry->first_cluster_high << 16)
                      | entry->first_cluster_low;
    cluster += read_offset;
    uint32_t remaining = entry->file_size;
    uint32_t offset = 0;

    if (remaining > buf_size) {
        // serial_puts("fat32: buffer too small for file\n");
        // return BUFFER_TOO_SMALL;
        remaining = buf_size;
    }

    while (remaining > 0) {
        uint32_t lba = cluster_to_lba(fs, cluster);
        uint32_t to_read = remaining < fs->cluster_size ? remaining : fs->cluster_size;

        int status = virtio_blk_read(
            fs->range->first_sector + lba,
            // virtio reads in full sectors, round up to nearest 512
            (to_read + 511) & ~511,
            buf + offset
        );
        if (status != 0) {
            serial_puts("fat32: failed to read file cluster\n");
            return IO_ERROR;
        }

        offset    += to_read;
        remaining -= to_read;

        if (remaining == 0) break;

        uint32_t next_cluster;
        status = read_fat_entry(fs, cluster, &next_cluster);
        if (status != 0)
            return status;

        if (next_cluster >= 0x0FFFFFF8) {
            // end of chain but remaining > 0 — corrupted FAT
            serial_puts("fat32: unexpected end of cluster chain\n");
            return IO_ERROR;
        }

        cluster = next_cluster;
    }

    return SUCCSESS;
}

int read_file(Fat32_Handle* fs, DirEntry* entry, uint8_t* buf, uint32_t buf_size) {
    if (entry->attr & 0x10) {
        serial_puts("fat32: entry is a directory, not a file\n");
        return NOT_A_FILE;
    }

    uint32_t cluster = ((uint32_t)entry->first_cluster_high << 16)
                      | entry->first_cluster_low;
    uint32_t remaining = entry->file_size;
    uint32_t offset = 0;

    if (remaining > buf_size) {
        // serial_puts("fat32: buffer too small for file\n");
        // return BUFFER_TOO_SMALL;
        remaining = buf_size;
    }

    while (remaining > 0) {
        uint32_t lba = cluster_to_lba(fs, cluster);
        uint32_t to_read = remaining < fs->cluster_size ? remaining : fs->cluster_size;

        int status = virtio_blk_read(
            fs->range->first_sector + lba,
            // virtio reads in full sectors, round up to nearest 512
            (to_read + 511) & ~511,
            buf + offset
        );
        if (status != 0) {
            serial_puts("fat32: failed to read file cluster\n");
            return IO_ERROR;
        }

        offset    += to_read;
        remaining -= to_read;

        if (remaining == 0) break;

        uint32_t next_cluster;
        status = read_fat_entry(fs, cluster, &next_cluster);
        if (status != 0)
            return status;

        if (next_cluster >= 0x0FFFFFF8) {
            // end of chain but remaining > 0 — corrupted FAT
            serial_puts("fat32: unexpected end of cluster chain\n");
            return IO_ERROR;
        }

        cluster = next_cluster;
    }

    return SUCCSESS;
}

int next_dir_entry(Fat32_Handle* fs, DirEntry** out_entry) {
    uint32_t entries_per_cluster = fs->cluster_size / sizeof(DirEntry);

    while (1) {
        while (fs->entry_index < entries_per_cluster) {
            DirEntry* e = (DirEntry*)(fs->cluster_buf + fs->entry_index * sizeof(DirEntry));
            fs->entry_index++;

            if (e->name[0] == 0x00)
                return END_OF_DIR;

            if (e->name[0] == 0xE5)
                continue;

            if (e->attr == 0x0F)
                continue;

            *out_entry = e;
            return SUCCSESS;
        }

        uint32_t next_cluster;
        int status = read_fat_entry(fs, fs->current_cluster, &next_cluster);
        if (status != 0)
            return status;

        if (next_cluster >= 0x0FFFFFF8)
            return END_OF_DIR;

        status = load_cluster(fs, next_cluster);
        if (status != 0)
            return status;
    }
}