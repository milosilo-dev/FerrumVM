#include "efi_loaded_image.h"

typedef struct {
    EFI_LOADED_IMAGE_PROTOCOL loaded_image;
} FAKE_IMAGE_HANDLE_DATA;