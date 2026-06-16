#include "headers/efi.h"
#include "headers/virtio_mmio.h"
#include "headers/virtqueue.h"
#include "headers/net.h"

static EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* con_out;

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

    // Init RX queue
    uint32_t rx_desc_addr  = (uint32_t)&rx_queue.desc;
    uint32_t rx_avail_addr = (uint32_t)&rx_queue.avail;
    uint32_t rx_used_addr = (uint32_t)&rx_queue.used;

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,     rx_desc_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,    0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,   rx_avail_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH,  0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,   rx_used_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH,  0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    // Init TX queue
    uint32_t tx_desc_addr  = (uint32_t)&tx_queue.desc;
    uint32_t tx_avail_addr = (uint32_t)&tx_queue.avail;
    uint32_t tx_used_addr = (uint32_t)&tx_queue.used;

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL,          1);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_LOW,     tx_desc_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DESC_HIGH,    0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_LOW,   tx_avail_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DRIVER_HIGH,  0);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_LOW,   tx_used_addr);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_DEVICE_HIGH,  0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY, 1);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE
        | VIRTIO_STATUS_DRIVER
        | VIRTIO_STATUS_FEATURES_OK
        | VIRTIO_STATUS_DRIVER_OK);
    
    virtio_net_config = *(VirtioNetConfig*)(VIRTIO_NET_BASE + 0x100);
}

// Read from device
//      buf must have space for a packet descriptor before the data section
int virtio_net_rx(uint8_t* buf, uint64_t length) {   
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);
    
    // Form a Virtio Descriptor with the packet as the payload
    uint32_t d = rx_next_desc % QUEUE_SIZE;
    rx_next_desc = (rx_next_desc + 1) % QUEUE_SIZE;

    rx_queue.desc[d].addr   = (uint64_t)(buf);
    rx_queue.desc[d].len    = length;
    rx_queue.desc[d].flags  = VIRTQ_DESC_F_WRITE;
    rx_queue.desc[d].next   = 0;

    // Put the Desc on the avail ring and nofiy the device
    rx_queue.avail.ring[rx_avail_idx % QUEUE_SIZE] = d;
    rx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    rx_queue.avail.idx = rx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        con_out->OutputString(con_out, L"virtio-net: queue not ready!\n");
    }

    while (rx_queue.used.idx == rx_last_used) {
        __asm__ volatile("pause" ::: "memory");
    }

    uint32_t written = rx_queue.used.ring[rx_last_used % QUEUE_SIZE].len;
    rx_last_used++;
    return 0;
}

// Write to device
//      buf must have a packet descriptor before the data section
int virtio_net_tx(uint8_t* buf, uint64_t length) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 1);

    // Form a Virtio Descriptor with the packet as the payload
    uint32_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    tx_queue.desc[d].addr   = (uint64_t)(buf);
    tx_queue.desc[d].len    = length;
    tx_queue.desc[d].flags  = 0;
    tx_queue.desc[d].next   = 0;

    // Put the Desc on the avail ring and nofiy the device
    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = d;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    uint32_t ready = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_READY);
    if (ready != 1) {
        con_out->OutputString(con_out, L"virtio-net: queue not ready!\n");
    }

    while (tx_queue.used.idx == tx_last_used) {
        __asm__ volatile("pause" ::: "memory");
    }

    uint32_t written = tx_queue.used.ring[tx_last_used % QUEUE_SIZE].len;
    tx_last_used++;
    return 0;
}

// Entry
EFI_STATUS EFIAPI efi_main(EFI_HANDLE* handle, EFI_SYSTEM_TABLE* st) {
    con_out = st->ConsoleOutHandle;
    con_out->OutputString(con_out, (CHAR16*)L"Test\n");

    for (;;) {
        __asm__ volatile("cli; hlt");
    }
}