// ── simple text input ─────────────────────────────────────────────
#include <stdint.h>
#include "uefi.h"

typedef struct {
    uint16_t ScanCode;
    uint16_t UnicodeChar;
} EFI_INPUT_KEY;

typedef struct _EFI_SIMPLE_TEXT_INPUT_PROTOCOL EFI_SIMPLE_TEXT_INPUT_PROTOCOL;
typedef EFI_STATUS (EFIAPI *EFI_INPUT_RESET)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL*, BOOLEAN);
typedef EFI_STATUS (EFIAPI *EFI_INPUT_READ_KEY)(EFI_SIMPLE_TEXT_INPUT_PROTOCOL*, EFI_INPUT_KEY*);

struct _EFI_SIMPLE_TEXT_INPUT_PROTOCOL {
    EFI_INPUT_RESET    Reset;
    EFI_INPUT_READ_KEY ReadKeyStroke;
    EFI_EVENT          WaitForKey;
};

static EFI_STATUS EFIAPI efi_con_in_reset(
    EFI_SIMPLE_TEXT_INPUT_PROTOCOL *This,
    BOOLEAN ExtendedVerification
) {
    serial_puts("[STUB] ConInReset");
    return EFI_SUCCESS;
}

static EFI_STATUS EFIAPI efi_read_key_stroke(
    EFI_SIMPLE_TEXT_INPUT_PROTOCOL *This,
    EFI_INPUT_KEY *Key
) {
    if (!serial_isdata()) {
        Key->ScanCode    = 0;
        Key->UnicodeChar = 0;
        serial_puts("[DBG] ReadKeyStroke: no data\n");
        return EFI_NOT_READY;
    }
    Key->ScanCode    = 0;
    Key->UnicodeChar = inb(COM1);
    serial_puts("[DBG] ReadKeyStroke: got char=");
    serial_putx(Key->UnicodeChar);
    serial_puts("\n");
    return EFI_SUCCESS;
}

static INTERNAL_EVENT gConInWaitEvent;

static EFI_SIMPLE_TEXT_INPUT_PROTOCOL gConIn = {
    .Reset         = efi_con_in_reset,
    .ReadKeyStroke = efi_read_key_stroke,
    .WaitForKey    = &gConInWaitEvent,
};