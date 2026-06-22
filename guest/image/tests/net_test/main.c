#include "headers/efi.h"
#include "headers/virtio_mmio.h"
#include "headers/virtqueue.h"
#include "headers/net.h"
#include <string.h>

static EFI_SIMPLE_TEXT_OUTPUT_PROTOCOL* con_out;

static volatile Virtqueue tx_queue __attribute__((aligned(4096)));
static uint16_t     tx_next_desc = 0;
static uint16_t     tx_avail_idx = 0;
static uint16_t     tx_last_used = 0;

static volatile Virtqueue rx_queue __attribute__((aligned(4096)));
static uint16_t     rx_next_desc = 0;
static uint16_t     rx_avail_idx = 0;
static uint16_t     rx_last_used = 0;

static inline void int_to_hex(unsigned int n, char *buffer) {
    static const char hex_chars[] = "0123456789ABCDEF";
    char temp[9];
    int i = 0;

    // Handle zero explicitly
    if (n == 0) {
        buffer[0] = '0';
        buffer[1] = '\0';
        return;
    }

    // Convert digits in reverse order
    while (n > 0) {
        temp[i++] = hex_chars[n & 0xF];
        n >>= 4;
    }

    // Reverse the string into the buffer
    buffer[i] = '\0';
    for (int j = 0; j < i; j++) {
        buffer[j] = temp[i - 1 - j];
    }
}

void print(const char *s) {
    CHAR16 wide[256];
    int i = 0;
    while (*s && i < 254) {
        if (*s == '\n') {
            wide[i++] = (CHAR16)'\r';
            wide[i++] = (CHAR16)'\n';
        } else {
            wide[i++] = (CHAR16)(*s);
        }
        s++;
    }
    wide[i] = 0;
    con_out->OutputString(con_out, wide);
}

void printx(uint32_t x) {
    char s[11];
    int_to_hex(x, s);
    CHAR16 wide[16];
    int i = 0;
    char *p = s;
    while (*p && i < 14) {
        wide[i++] = (CHAR16)(*p++);
    }
    wide[i] = 0;
    con_out->OutputString(con_out, wide);
}

int virtio_net_init(void){
    // ---- Validate device ----
    uint32_t magic = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_MAGIC);
    uint32_t version = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_VERSION);
    uint32_t dev_id = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_DEVICE_ID);
    uint32_t vendor = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_VENDOR_ID);

    print("magic="); printx(magic);
    print(" ver="); printx(version);
    print(" dev="); printx(dev_id);
    print(" vendor="); printx(vendor);

    if (magic != 0x74726976) { print(" BAD MAGIC\n"); return -1; }
    if (version != 2)       { print(" BAD VER\n");   return -1; }
    if (dev_id != 1)        { print(" NOT NET\n");   return -1; }
    print(" OK\n");

    // ---- Validate read/write to device ----
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    uint32_t rw_test = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS);
    if (!(rw_test & VIRTIO_STATUS_ACKNOWLEDGE)) {
        print("STATUS wr/rd FAIL\n");
        return -1;
    }
    print("STATUS wr/rd OK\n");

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, 0);

    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER);

    uint32_t features = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_DEVICE_FEATURES);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_DRVR_FEATURES, features);
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS, VIRTIO_STATUS_ACKNOWLEDGE | VIRTIO_STATUS_DRIVER | VIRTIO_STATUS_FEATURES_OK);
    uint32_t status = mmio_read(VIRTIO_NET_BASE, VIRTIO_MMIO_STATUS);

    if (!(status & VIRTIO_STATUS_FEATURES_OK)) {
        print("FEATURES REJECTED\n");
        return -1;
    }

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
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NUM, QUEUE_SIZE);
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
    return 0;
}

static int wait_used(const volatile VirtqUsed *used, uint16_t *last, uint32_t timeout) {
    for (uint32_t i = 0; i < timeout; i++) {
        __asm__ volatile("pause");
        if (used->idx != *last) {
            *last = used->idx;
            return 1;
        }
    }
    return 0;
}

// Write to device
//      buf[0..11] = virtio-net header (zeros OK), buf[12..] = ethernet frame
static int virtio_net_tx_naked(uint8_t* buf, uint64_t length, int kick);

int virtio_net_tx(uint8_t* buf, uint64_t length) {
    return virtio_net_tx_naked(buf, length, 1);
}

// Same as above but optionally suppress the kick to test tick-only path
static int virtio_net_tx_naked(uint8_t* buf, uint64_t length, int kick) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 1);

    uint32_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    tx_queue.desc[d].addr   = (uint64_t)(buf);
    tx_queue.desc[d].len    = length;
    tx_queue.desc[d].flags  = 0;
    tx_queue.desc[d].next   = 0;

    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = d;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    if (kick)
        mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    if (!wait_used(&tx_queue.used, &tx_last_used, 10000000)) {
        print("TX timeout\n");
        return -1;
    }
    return 0;
}

// Push a TX descriptor but do NOT wait — used for batch submission.
static void virtio_net_tx_submit(uint8_t* buf, uint64_t length) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 1);

    uint32_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    tx_queue.desc[d].addr   = (uint64_t)(buf);
    tx_queue.desc[d].len    = length;
    tx_queue.desc[d].flags  = 0;
    tx_queue.desc[d].next   = 0;

    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = d;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
}

// ---- Chained-descriptor TX (like Linux does) ----
// Submit TX with header and data in separate chained descriptors:
//   desc[h]: 12-byte virtio-net header (flags = NEXT, next = h+1)
//   desc[h+1]: ethernet frame data (flags = 0)
static int virtio_net_tx_chained(uint8_t* hdr_buf, uint8_t* data_buf, uint64_t data_len) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 1);

    uint32_t h = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;
    uint32_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    // Descriptor for 12-byte virtio-net header, chains to data
    tx_queue.desc[h].addr   = (uint64_t)(hdr_buf);
    tx_queue.desc[h].len    = 12;
    tx_queue.desc[h].flags  = VIRTQ_DESC_F_NEXT;
    tx_queue.desc[h].next   = d;

    // Descriptor for ethernet frame data
    tx_queue.desc[d].addr   = (uint64_t)(data_buf);
    tx_queue.desc[d].len    = data_len;
    tx_queue.desc[d].flags  = 0;
    tx_queue.desc[d].next   = 0;

    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = h;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    if (!wait_used(&tx_queue.used, &tx_last_used, 10000000)) {
        print("TX chained timeout\n");
        return -1;
    }
    return 0;
}

// Submit TX with header + 2 data fragments (3 chained descriptors):
//   desc[h]: header (NEXT -> h+1)
//   desc[d1]: data frag 1 (NEXT -> d2)
//   desc[d2]: data frag 2 (flags=0)
static int virtio_net_tx_triple_chain(uint8_t* hdr_buf, uint8_t* data1_buf, uint64_t data1_len, uint8_t* data2_buf, uint64_t data2_len) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 1);

    uint32_t h  = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;
    uint32_t d1 = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;
    uint32_t d2 = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    tx_queue.desc[h].addr   = (uint64_t)(hdr_buf);
    tx_queue.desc[h].len    = 12;
    tx_queue.desc[h].flags  = VIRTQ_DESC_F_NEXT;
    tx_queue.desc[h].next   = d1;

    tx_queue.desc[d1].addr  = (uint64_t)(data1_buf);
    tx_queue.desc[d1].len   = data1_len;
    tx_queue.desc[d1].flags = VIRTQ_DESC_F_NEXT;
    tx_queue.desc[d1].next  = d2;

    tx_queue.desc[d2].addr  = (uint64_t)(data2_buf);
    tx_queue.desc[d2].len   = data2_len;
    tx_queue.desc[d2].flags = 0;
    tx_queue.desc[d2].next  = 0;

    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = h;
    tx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    if (!wait_used(&tx_queue.used, &tx_last_used, 10000000)) {
        print("TX triple-chain timeout\n");
        return -1;
    }
    return 0;
}

// Trigger a single kick and wait for all outstanding TX to drain.
static int virtio_net_tx_drain(void) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);
    if (!wait_used(&tx_queue.used, &tx_last_used, 10000000)) {
        print("TX drain timeout\n");
        return -1;
    }
    return 0;
}

// Read from device
int virtio_net_rx(uint8_t* buf, uint64_t length) {
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_SEL, 0);

    uint32_t d = rx_next_desc % QUEUE_SIZE;
    rx_next_desc = (rx_next_desc + 1) % QUEUE_SIZE;

    rx_queue.desc[d].addr   = (uint64_t)(buf);
    rx_queue.desc[d].len    = length;
    rx_queue.desc[d].flags  = VIRTQ_DESC_F_WRITE;
    rx_queue.desc[d].next   = 0;

    rx_queue.avail.ring[rx_avail_idx % QUEUE_SIZE] = d;
    rx_avail_idx++;
    __asm__ volatile("" ::: "memory");
    rx_queue.avail.idx = rx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 0);

    if (!wait_used(&rx_queue.used, &rx_last_used, 10000000)) {
        print("RX timeout last_used=");
        printx(rx_last_used);
        print("\n");
        return -1;
    }
    uint32_t written = rx_queue.used.ring[(rx_last_used - 1) % QUEUE_SIZE].len;
    return written;
}

static void fill_ether_frame(uint8_t *buf, unsigned seq) {
    memset(buf, 0, 128);
    // 12-byte virtio-net header (all zeros = no offloads)
    // Then minimum ethernet frame: dst MAC + src MAC + type
    buf[12] = 0xFF; buf[13] = 0xFF; buf[14] = 0xFF; // dst broadcast
    buf[15] = 0xFF; buf[16] = 0xFF; buf[17] = 0xFF;
    buf[18] = 0x52; buf[19] = 0x54; buf[20] = 0x00; // src = device MAC
    buf[21] = 0x12; buf[22] = 0x34; buf[23] = 0x56;
    buf[24] = 0x08; buf[25] = 0x00;                   // EtherType IPv4
    buf[26] = (uint8_t)(seq >> 24);
    buf[27] = (uint8_t)(seq >> 16);
    buf[28] = (uint8_t)(seq >>  8);
    buf[29] = (uint8_t)(seq);
}

static int test_tx_basic(void) {
    uint8_t tx_buf[128];
    fill_ether_frame(tx_buf, 0);
    print("  TX single: ");
    if (virtio_net_tx(tx_buf, 72) < 0) {
        print("FAIL\n");
        return -1;
    }
    print("OK\n");
    return 0;
}

static int test_tx_multi(int count) {
    print("  TX multi");
    printx(count);
    print(": ");
    for (int i = 0; i < count; i++) {
        uint8_t tx_buf[128];
        fill_ether_frame(tx_buf, i);
        if (virtio_net_tx(tx_buf, 72) < 0) {
            print("FAIL at ");
            printx(i);
            print("\n");
            return -1;
        }
    }
    print("OK\n");
    return 0;
}

static int test_tx_batch(int count) {
    print("  TX batch");
    printx(count);
    print(": ");
    for (int i = 0; i < count; i++) {
        uint8_t tx_buf[128];
        fill_ether_frame(tx_buf, i);
        virtio_net_tx_submit(tx_buf, 72);
    }
    if (virtio_net_tx_drain() < 0) {
        print("FAIL\n");
        return -1;
    }
    print("OK\n");
    return 0;
}

static int test_tx_no_kick(void) {
    print("  TX no-kick: ");
    uint8_t tx_buf[128];
    fill_ether_frame(tx_buf, 0xdead);
    if (virtio_net_tx_naked(tx_buf, 72, 0) < 0) {
        print("FAIL\n");
        return -1;
    }
    print("OK\n");
    return 0;
}

static int test_tx_chained(void) {
    print("  TX chained: ");
    uint8_t hdr_buf[12] __attribute__((aligned(16)));
    uint8_t data_buf[128] __attribute__((aligned(16)));
    memset(hdr_buf, 0, 12);
    memset(data_buf, 0, 128);
    // Full ethernet frame (dst + src + EtherType + payload) in data buf
    data_buf[0] = 0xFF; data_buf[1] = 0xFF; data_buf[2] = 0xFF;
    data_buf[3] = 0xFF; data_buf[4] = 0xFF; data_buf[5] = 0xFF;
    data_buf[6] = 0x52; data_buf[7] = 0x54;
    data_buf[8] = 0x00; data_buf[9] = 0x12; data_buf[10] = 0x34; data_buf[11] = 0x56;
    data_buf[12] = 0x08; data_buf[13] = 0x00;
    data_buf[14] = 0xCA; data_buf[15] = 0xFE;  // magic bytes
    if (virtio_net_tx_chained(hdr_buf, data_buf, 60) < 0) {
        print("FAIL\n");
        return -1;
    }
    print("OK\n");
    return 0;
}

static int test_tx_triple_chain(void) {
    print("  TX triple-chain: ");
    uint8_t hdr_buf[12] __attribute__((aligned(16)));
    uint8_t data1_buf[64] __attribute__((aligned(16)));
    uint8_t data2_buf[64] __attribute__((aligned(16)));
    memset(hdr_buf, 0, 12);
    memset(data1_buf, 0, 64);
    memset(data2_buf, 0, 64);
    // First fragment: dst MAC + src MAC + EtherType
    data1_buf[0] = 0xFF; data1_buf[1] = 0xFF; data1_buf[2] = 0xFF;
    data1_buf[3] = 0xFF; data1_buf[4] = 0xFF; data1_buf[5] = 0xFF;
    data1_buf[6] = 0x52; data1_buf[7] = 0x54;
    data1_buf[8] = 0x00; data1_buf[9] = 0x12; data1_buf[10] = 0x34; data1_buf[11] = 0x56;
    data1_buf[12] = 0x08; data1_buf[13] = 0x00; // EtherType IPv4
    // Second fragment: payload
    data2_buf[0] = 0xDE; data2_buf[1] = 0xAD;
    data2_buf[2] = 0xBE; data2_buf[3] = 0xEF;
    if (virtio_net_tx_triple_chain(hdr_buf, data1_buf, 14, data2_buf, 46) < 0) {
        print("FAIL\n");
        return -1;
    }
    print("OK\n");
    return 0;
}

// Entry
EFI_STATUS EFIAPI efi_main(EFI_HANDLE* handle, EFI_SYSTEM_TABLE* st) {
    int failures = 0;

    con_out = st->ConOut;
    con_out->SetCursorPosition();
    con_out->OutputString(con_out, (CHAR16*)L"--- Ferrum Network Test Utility ---\n\r");

    if (virtio_net_init() < 0) {
        print("virtio-net init FAILED\n");
        for (;;) { __asm__ volatile("cli; hlt"); }
    }

    print("--- TX tests ---\n");
    failures += test_tx_basic();
    failures += test_tx_multi(3);
    failures += test_tx_multi(10);
    failures += test_tx_batch(5);
    failures += test_tx_no_kick();
    failures += test_tx_chained();
    failures += test_tx_triple_chain();

    // ---- RX test ----
    uint8_t rx_buf[128];
    memset(rx_buf, 0, 128);

    print("--- RX test ---\n");
    print("  RX: ");
    int n = virtio_net_rx(rx_buf, 128);
    if (n < 0)
        print("no packet\n");
    else {
        print("got ");
        printx(n);
        print(" bytes\n");
    }

    print("\n--- Summary: ");
    if (failures) {
        printx(failures);
        print(" test(s) FAILED\n");
    } else {
        print("ALL OK\n");
    }

    for (;;) {
        __asm__ volatile("cli; hlt");
    }
}