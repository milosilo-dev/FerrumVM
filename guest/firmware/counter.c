#include "headers/serial.h"
#include "headers/virtio_mmio.h"
#include "headers/virtqueue.h"
#include <stdint.h>
#include <stdbool.h>

static Virtqueue cnt_queue __attribute__((aligned(4096)));
static uint16_t  cnt_next_desc = 0;
static uint16_t  cnt_avail_idx = 0;
static uint16_t cnt_last_used = 0;

void virtio_cnt_init(void){
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_STATUS, 0);

    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    uint32_t features = mmio_read(VIRTIO_CNT_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_DRVR_FEATURES, features);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);

    // Pointers to the memory holding the respective parts of the queue
    uint32_t desc_addr  = (uint32_t)&cnt_queue.desc;
    uint32_t avail_addr = (uint32_t)&cnt_queue.avail;
    uint32_t used_addr = (uint32_t)&cnt_queue.used;

    // Fill the locations at the pointers with the correct values
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,    desc_addr);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,   0);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,  avail_addr);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH, 0);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,  used_addr);
    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH, 0);

    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);

    serial_puts("virtio-cnt: init done\n");
}

typedef struct {
    uint32_t input;
    uint32_t output;
} VirtioCountData;

uint32_t virtio_cnt(uint32_t in) {
    VirtioCountData data;
    data.input = in;

    uint16_t d = cnt_next_desc % QUEUE_SIZE;
    uint16_t d2 = (cnt_next_desc + 1) % QUEUE_SIZE;
    cnt_next_desc = (cnt_next_desc + 2) % QUEUE_SIZE;

    cnt_queue.desc[d].addr  = (uint32_t)&data.input;
    cnt_queue.desc[d].len   = 4;
    cnt_queue.desc[d].flags = VIRTQ_DESC_F_WRITE | VIRTQ_DESC_F_NEXT;
    cnt_queue.desc[d].next  = d2;

    cnt_queue.desc[d2].addr = (uint32_t)&data.output;
    cnt_queue.desc[d2].len   = 4;
    cnt_queue.desc[d2].flags = VIRTQ_DESC_F_WRITE;
    cnt_queue.desc[d2].next  = 0;

    cnt_queue.avail.ring[cnt_avail_idx % QUEUE_SIZE] = d;
    cnt_avail_idx++;
    __asm__ volatile("" ::: "memory");
    cnt_queue.avail.idx = cnt_avail_idx;

    mmio_write(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_CNT_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        serial_puts("virtio-cnt: queue not ready!\n");
    }

    while (cnt_queue.used.idx == cnt_last_used) {
        __asm__ volatile("pause");
    }

    uint32_t written = cnt_queue.used.ring[cnt_last_used % QUEUE_SIZE].len;
    cnt_last_used++;

    return data.output;
}