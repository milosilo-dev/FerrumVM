#include "headers/efi.h"
#include "headers/virtio_mmio.h"
#include "headers/virtqueue.h"

static volatile Virtqueue tx_queue __attribute__((aligned(4096)));
static uint16_t     tx_next_desc = 0;
static uint16_t     tx_avail_idx = 0;
static uint16_t     tx_last_used = 0;

static volatile Virtqueue rx_queue __attribute__((aligned(4096)));
static uint16_t     rx_next_desc = 0;
static uint16_t     rx_avail_idx = 0;
static uint16_t     rx_last_used = 0;

void virtio_net_init(void){
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, 0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    uint32_t features = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_DRVR_FEATURES, features);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);

    // Pointers to the memory holding the respective parts of the queue
    uint32_t desc_addr  = (uint32_t)&blk_queue.desc;
    uint32_t avail_addr = (uint32_t)&blk_queue.avail;
    uint32_t used_addr = (uint32_t)&blk_queue.used;

    // Fill the locations at the pointers with the correct values
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,    desc_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,   0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,  avail_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH, 0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,  used_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH, 0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);
    
    virtio_net_config = *(VirtioNetConfig*)(VIRTIO_NET_BASE + 0x100);
}