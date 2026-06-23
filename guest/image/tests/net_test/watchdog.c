static void wd_submit_one(uint8_t *buf, uint64_t length) {
    uint32_t d = tx_next_desc % QUEUE_SIZE;
    tx_next_desc = (tx_next_desc + 1) % QUEUE_SIZE;

    tx_queue.desc[d].addr  = (uint64_t)(buf);
    tx_queue.desc[d].len   = length;
    tx_queue.desc[d].flags = 0;
    tx_queue.desc[d].next  = 0;

    tx_queue.avail.ring[tx_avail_idx % QUEUE_SIZE] = d;
    tx_avail_idx++;
}

static void wd_fill_frame(uint8_t *buf, unsigned seq) {
    memset(buf, 0, 128);
    buf[12] = 0xFF; buf[13] = 0xFF; buf[14] = 0xFF;
    buf[15] = 0xFF; buf[16] = 0xFF; buf[17] = 0xFF;
    buf[18] = 0x52; buf[19] = 0x54; buf[20] = 0x00;
    buf[21] = 0x12; buf[22] = 0x34; buf[23] = 0x56;
    buf[24] = 0x08; buf[25] = 0x00;
    buf[26] = (uint8_t)(seq >> 24);
    buf[27] = (uint8_t)(seq >> 16);
    buf[28] = (uint8_t)(seq >>  8);
    buf[29] = (uint8_t)(seq);
}

/* Busy-wait without ever publishing avail.idx in between submissions.
 * Returns 1 if used ring eventually advanced to match what we expect,
 * 0 on timeout (this is the "reproduced the hang" case). */
static int wd_wait_used_count(uint16_t expect_advance, uint32_t timeout) {
    uint16_t start = tx_last_used;
    for (uint32_t i = 0; i < timeout; i++) {
        __asm__ volatile("pause");
        uint16_t cur = tx_queue.used.idx;
        if ((uint16_t)(cur - start) >= expect_advance) {
            tx_last_used = cur;
            return 1;
        }
    }
    return 0;
}

/* ---- Test A: single oversized flood, one kick, one wait ----
 * Queues 2*QUEUE_SIZE descriptors in a single burst (more than the ring
 * can structurally hold without the host draining at least once), then
 * publishes avail.idx ONCE and kicks ONCE. If the host's pop_avail() ever
 * computes entries_available > size on its first read of this jump, the
 * whole batch is silently dropped on the host side and no used entries
 * ever appear. We'll see our own wait_used() time out exactly the way
 * the kernel's TX watchdog would.
 *
 * NOTE: this writes more descriptor *slots* than QUEUE_SIZE by wrapping
 * tx_next_desc, which means later submissions overwrite earlier
 * descriptor table entries before the host has read them. This is
 * intentionally hostile -- it's exactly what a guest under heavy load
 * piling up TX faster than the host drains can produce.
 */
static int test_tx_watchdog_flood(int multiplier) {
    print("  TX watchdog flood x");
    printx((uint32_t)multiplier);
    print(": ");

    uint16_t before_used = tx_queue.used.idx;
    uint16_t before_avail = tx_avail_idx;

    static uint8_t bufs[64][128] __attribute__((aligned(16)));
    int n = QUEUE_SIZE * multiplier;
    if (n > 64) n = 64; /* cap to our static buffer pool */

    for (int i = 0; i < n; i++) {
        wd_fill_frame(bufs[i], (unsigned)i);
        wd_submit_one(bufs[i], 72);
    }

    /* Publish once, kick once -- mirrors a guest that batches under load
     * instead of notifying per-packet. */
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    uint16_t expect_advance = (uint16_t)(tx_avail_idx - before_avail);
    if (!wd_wait_used_count(expect_advance, 10000000)) {
        print("HANG REPRODUCED (used.idx stuck at ");
        printx(tx_queue.used.idx);
        print(", expected advance ");
        printx(expect_advance);
        print(" from ");
        printx(before_used);
        print(")\n");
        return -1;
    }
    print("OK (drained fully, did not reproduce)\n");
    return 0;
}

/* ---- Test B: double flood, no intermediate wait ----
 * Same as above, but fires the flood-and-kick sequence twice back to
 * back with NO wait_used() call between them, so if the host falls
 * behind (e.g. its tick() hasn't run since the last notify, or it's
 * processing something else), the guest's avail_idx can run far ahead
 * of the host's last_avail_idx -- which is the exact precondition for
 * entries_available (computed as a u16 wrapping_sub) to overflow past
 * `size` on the host's NEXT pop_avail() call.
 */
static int test_tx_watchdog_double_flood(int multiplier) {
    print("  TX watchdog double-flood x");
    printx((uint32_t)multiplier);
    print(": ");

    uint16_t before_used = tx_queue.used.idx;
    uint16_t before_avail = tx_avail_idx;

    static uint8_t bufs[64][128] __attribute__((aligned(16)));
    int n = QUEUE_SIZE * multiplier;
    if (n > 64) n = 64;

    /* Round 1: submit + publish + kick, but do NOT wait. */
    for (int i = 0; i < n; i++) {
        wd_fill_frame(bufs[i], (unsigned)i);
        wd_submit_one(bufs[i], 72);
    }
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    /* Round 2: immediately do it again, same buffers (descs will alias
     * over what we just queued -- that's fine, we're stressing index
     * math, not data integrity here). */
    for (int i = 0; i < n; i++) {
        wd_fill_frame(bufs[i], (unsigned)(i + 1000));
        wd_submit_one(bufs[i], 72);
    }
    __asm__ volatile("" ::: "memory");
    tx_queue.avail.idx = tx_avail_idx;
    virtio_mb();
    mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);

    uint16_t expect_advance = (uint16_t)(tx_avail_idx - before_avail);
    if (!wd_wait_used_count(expect_advance, 10000000)) {
        print("HANG REPRODUCED (used.idx stuck at ");
        printx(tx_queue.used.idx);
        print(", expected advance ");
        printx(expect_advance);
        print(" from ");
        printx(before_used);
        print(")\n");
        return -1;
    }
    print("OK (drained fully, did not reproduce)\n");
    return 0;
}

/* ---- Test C: wrap-exact ----
 * Drive tx_avail_idx to sit just before a u16 wrap (0xFFFF -> 0x0000),
 * then submit a flood that crosses the wrap boundary. This targets any
 * place that compares raw indices instead of using wrapping/modular
 * arithmetic consistently between guest and host.
 *
 * We get there by submitting+kicking+waiting single packets in a loop
 * (cheap, draining each time so we don't hang ourselves) until
 * tx_avail_idx is within `approach` of wrapping, then do the real flood
 * test across the wrap point.
 */
static int test_tx_watchdog_wrap(void) {
    print("  TX watchdog wrap-boundary: ");

    uint8_t buf[128] __attribute__((aligned(16)));

    /* Fast-forward tx_avail_idx close to 0xFFFF by doing single
     * submit+kick+wait cycles. This does NOT use virtio_net_tx() so we
     * keep full control, but it follows the same single-packet protocol
     * so it's safe (host should drain each one). */
    while (tx_avail_idx < (uint16_t)0xFFF0) {
        wd_fill_frame(buf, 0xAAAA);
        uint16_t before = tx_queue.used.idx;
        wd_submit_one(buf, 72);
        __asm__ volatile("" ::: "memory");
        tx_queue.avail.idx = tx_avail_idx;
        virtio_mb();
        mmio_write(VIRTIO_NET_BASE, VIRTIO_MMIO_QUEUE_NOTIFY, 1);
        if (!wd_wait_used_count(1, 10000000)) {
            print("setup phase hung early at idx=");
            printx(tx_avail_idx);
            print(" (interesting on its own!)\n");
            return -1;
        }
        (void)before;
    }

    print("at idx=");
    printx(tx_avail_idx);
    print(", flooding across wrap: ");

    /* Now flood across the 0xFFFF -> 0x0000 boundary. */
    return test_tx_watchdog_flood(4) == 0 ? 0 : -1;
}
