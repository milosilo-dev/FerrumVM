// net.c
#include "net.h"
#include "../headers/serial.h"
#include "../headers/virtio_mmio.h"
#include "../headers/virtqueue.h"
#include "../mem/heap.h"
#include <stdint.h>

#define NET_RX_QUEUE_IDX 0
#define NET_TX_QUEUE_IDX 1

// Max Ethernet frame (no offloads) + virtio_net_hdr, matches spec's
// "buffers of at least 1526 bytes" requirement for non-MRG_RXBUF mode.
#define NET_BUF_SIZE (sizeof(VirtioNetHdr) + 1514)

static volatile Virtqueue net_rx_queue __attribute__((aligned(4096)));
static volatile Virtqueue net_tx_queue __attribute__((aligned(4096)));

static uint16_t rx_avail_idx = 0;
static uint16_t rx_last_used = 0;
static uint16_t tx_next_desc = 0;
static uint16_t tx_avail_idx = 0;
static uint16_t tx_last_used = 0;

VirtioNetConfig virtio_net_config = {0};

// Backing storage for every rx descriptor's buffer, and one tx buffer
// reused synchronously per send (same lifetime assumption blk.c makes
// with its static `request`: safe because we busy-poll for completion
// before returning, so nothing else touches it in the meantime).
static uint8_t net_rx_buffers[QUEUE_SIZE][NET_BUF_SIZE] __attribute__((aligned(16)));
static uint8_t net_tx_buffer[NET_BUF_SIZE];

// Posts an rx descriptor pointing at net_rx_buffers[idx] as an
// empty, device-writable buffer, and exposes it via the avail ring.
static void net_rx_post_buffer(uint16_t idx) {
    net_rx_queue.desc[idx].addr  = (uint64_t)&net_rx_buffers[idx];
    net_rx_queue.desc[idx].len   = NET_BUF_SIZE;
    net_rx_queue.desc[idx].flags = VIRTQ_DESC_F_WRITE;
    net_rx_queue.desc[idx].next  = 0;

    net_rx_queue.avail.ring[rx_avail_idx % QUEUE_SIZE] = idx;
    rx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    net_rx_queue.avail.idx = rx_avail_idx;
}

static void net_setup_queue(uint16_t queue_idx, volatile Virtqueue* q) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, queue_idx);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);

    uint32_t desc_addr  = (uint32_t)&q->desc;
    uint32_t avail_addr = (uint32_t)&q->avail;
    uint32_t used_addr  = (uint32_t)&q->used;

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,    desc_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,   0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,  avail_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH, 0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,  used_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH, 0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY, 1);
}

void virtio_net_init(void) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, 0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    // Negotiate nothing: no CSUM, no MRG_RXBUF, no MAC, no CTRL_VQ.
    // Simplest possible driver — one descriptor per packet, header
    // fully zeroed, MAC/link status just left unread.
    (void)mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_DRVR_FEATURES, 0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS,
        VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);

    uint32_t status = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS);
    if (!(status & VIRTIO_STATUS_FEATURES_OK)) {
        serial_puts("virtio-net: device rejected feature set!\n");
        return;
    }

    net_setup_queue(NET_RX_QUEUE_IDX, &net_rx_queue);
    net_setup_queue(NET_TX_QUEUE_IDX, &net_tx_queue);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);

    virtio_net_config = *(VirtioNetConfig*)(VIRTIO_NET_BASE + 0x100);

    // Pre-fill the whole rx ring with empty buffers so the device has
    // somewhere to write incoming packets from the start.
    for (uint16_t i = 0; i < QUEUE_SIZE; i++) {
        net_rx_post_buffer(i);
    }
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, NET_RX_QUEUE_IDX);
}

uint8_t virtio_net_send(const uint8_t* buf, uint16_t length) {
    if (length + sizeof(VirtioNetHdr) > NET_BUF_SIZE) {
        serial_puts("virtio-net: frame too large!\n");
        return 1;
    }

    VirtioNetHdr* hdr = (VirtioNetHdr*)net_tx_buffer;
    memset(hdr, 0, sizeof(VirtioNetHdr));
    memcpy(net_tx_buffer + sizeof(VirtioNetHdr), buf, length);

    uint16_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    net_tx_queue.desc[d].addr  = (uint64_t)net_tx_buffer;
    net_tx_queue.desc[d].len   = sizeof(VirtioNetHdr) + length;
    net_tx_queue.desc[d].flags = 0; // device-readable, no chaining
    net_tx_queue.desc[d].next  = 0;

    net_tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = d;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    net_tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, NET_TX_QUEUE_IDX);

    while (net_tx_queue.used.idx == tx_last_used) {
        __asm__ volatile("pause" ::: "memory");
    }
    tx_last_used++;

    return 0;
}

uint16_t virtio_net_poll_receive(uint8_t* buf, uint16_t buf_cap) {
    if (net_rx_queue.used.idx == rx_last_used) {
        return 0; // nothing new
    }

    VirtqUsedElem e = net_rx_queue.used.ring[rx_last_used % QUEUE_SIZE];
    rx_last_used++;

    uint16_t desc_idx = (uint16_t)e.id;
    uint32_t written  = e.len; // hdr + frame bytes the device actually wrote

    if (written < sizeof(VirtioNetHdr)) {
        serial_puts("virtio-net: short rx descriptor!\n");
        net_rx_post_buffer(desc_idx); // recycle and move on
        virtio_mb();
        mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, NET_RX_QUEUE_IDX);
        return 0;
    }

    uint32_t frame_len = written - sizeof(VirtioNetHdr);
    if (frame_len > buf_cap) {
        frame_len = buf_cap; // truncate rather than overflow caller's buffer
    }
    memcpy(buf, net_rx_buffers[desc_idx] + sizeof(VirtioNetHdr), frame_len);

    // Recycle this buffer back onto the avail ring for reuse.
    net_rx_post_buffer(desc_idx);
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, NET_RX_QUEUE_IDX);

    return (uint16_t)frame_len;
}

void virtio_net_dump(void) {}