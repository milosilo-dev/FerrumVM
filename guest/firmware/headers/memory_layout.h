#ifndef MEMORY_LAYOUT_H
#define MEMORY_LAYOUT_H

/* Central physical-address layout shared across the firmware. */

/* File-buffer region used to stage loaded EFI applications. */
#define FILE_BUF_ADDR   0x1000000ULL

/* Heap region managed by init_heap(). */
#define HEAP_START      0x3000000ULL
#define HEAP_END        0x4000000ULL

/* Host-injected memory map descriptor table. */
#define MEMMAP_ADDR     0x7000ULL

#endif
