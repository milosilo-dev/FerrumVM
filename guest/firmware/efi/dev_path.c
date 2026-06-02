#include "dev_path.h"

static EFI_DEVICE_PATH_PROTOCOL gDevicePathData = {
    0x7F,  // END_DEVICE_PATH_TYPE
    0xFF,  // END_ENTIRE_DEVICE_PATH_SUBTYPE
    {4,0}
};
EFI_HANDLE gDevicePath = (EFI_HANDLE)&gDevicePathData;

EFI_GUID gEfiDevicePathProtocolGuid = {
    0x09576e91,0x6d3f,0x11d2,
    {0x8e,0x39,0x00,0xa0,0xc9,0x69,0x72,0x3b}
};