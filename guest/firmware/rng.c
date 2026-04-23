#include "serial.h"
#include "virtio_mmio.h"
#include "virtqueue.h"
#include <stdint.h>

static Virtqueue rng_queue __attribute__((aligned(4096)));
static uint16_t  rng_next_desc = 0;
static uint16_t  rng_avail_idx = 0;
static uint16_t rng_last_used = 0;

void virtio_rng_init(void){
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_STATUS, 0);

    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    uint32_t features = mmio_read(VIRTIO_RNG_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_DRVR_FEATURES, features);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);

    // Pointers to the memory holding the respective parts of the queue
    uint32_t desc_addr  = (uint64_t)&rng_queue.desc;
    uint32_t avail_addr = (uint64_t)&rng_queue.avail;
    uint32_t used_addr = (uint64_t)&rng_queue.used;

    // Fill the locations at the pointers with the correct values
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,    desc_addr);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,   0);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,  avail_addr);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH, 0);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,  used_addr);
    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH, 0);

    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);

    serial_puts("virtio-rng: init done\n");
}

uint32_t virtio_rng_read(uint8_t *buf, uint32_t len) {
    uint16_t d = rng_next_desc % QUEUE_SIZE;
    rng_next_desc = (rng_next_desc + 1) % QUEUE_SIZE;

    rng_queue.desc[d].addr  = (uint64_t)buf;
    rng_queue.desc[d].len   = len;
    rng_queue.desc[d].flags = VIRTQ_DESC_F_WRITE;
    rng_queue.desc[d].next  = 0;

    rng_queue.avail.ring[rng_avail_idx % QUEUE_SIZE] = d;
    rng_avail_idx++;
    __asm__ volatile("" ::: "memory");
    rng_queue.avail.idx = rng_avail_idx;

    mmio_write(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_RNG_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        serial_puts("virtio-rng: queue not ready!\n");
    }

    while (rng_queue.used.idx == rng_last_used) {
        __asm__ volatile("pause");
    }

    uint32_t written = rng_queue.used.ring[rng_last_used % QUEUE_SIZE].len;
    rng_last_used++;

    return written;
}