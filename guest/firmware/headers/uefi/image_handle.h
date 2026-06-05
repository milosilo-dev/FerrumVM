#include "efi_loaded_image.h"

typedef struct {
    uint64_t                  magic;
    EFI_LOADED_IMAGE_PROTOCOL loaded_image;
} EFI_IMAGE_HANDLE_DATA;