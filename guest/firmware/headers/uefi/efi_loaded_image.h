#ifndef EFI_LOADED_IMAGE_H
#define EFI_LOADED_IMAGE_H

#include "uefi.h"

#define EFI_LOADED_IMAGE_PROTOCOL_GUID \
    { 0x5B1B31A1, 0x9562, 0x11D2, \
      { 0x8E, 0x3F, 0x00, 0xA0, 0xC9, 0x69, 0x72, 0x3B } }

#define EFI_LOADED_IMAGE_PROTOCOL_REVISION 0x1000

typedef EFI_STATUS (EFIAPI *EFI_IMAGE_UNLOAD)(
    EFI_HANDLE ImageHandle
);

typedef struct {
    UINT32                    Revision;        /* Must be EFI_LOADED_IMAGE_PROTOCOL_REVISION */
    EFI_HANDLE                ParentHandle;    /* Handle of the caller that loaded this image */
    EFI_SYSTEM_TABLE         *SystemTable;     /* Pointer to the EFI system table */

    /* Source location of the image */
    EFI_HANDLE                DeviceHandle;    /* Device handle image was loaded from */
    EFI_DEVICE_PATH_PROTOCOL *FilePath;        /* File path of the image on the device */
    VOID                     *Reserved;        /* Reserved, must be NULL */

    /* Load options */
    UINT32                    LoadOptionsSize; /* Size in bytes of LoadOptions */
    VOID                     *LoadOptions;     /* Pointer to the image's load options */

    /* Memory location of the loaded image */
    VOID                     *ImageBase;       /* Base address of the loaded image in memory */
    uint64_t                    ImageSize;        /* Size in bytes of the loaded image */
    EFI_MEMORY_TYPE           ImageCodeType;   /* Memory type image code is allocated from */
    EFI_MEMORY_TYPE           ImageDataType;   /* Memory type image data is allocated from */

    /* Unload callback */
    EFI_IMAGE_UNLOAD          Unload;          /* Callback for unloading the image */

} EFI_LOADED_IMAGE_PROTOCOL;

#endif /* EFI_LOADED_IMAGE_H */