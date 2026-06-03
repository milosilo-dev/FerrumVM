#pragma once
#include "../../efi/dev_path.h"

typedef struct {
    EFI_DEVICE_PATH_PROTOCOL FileNode;
    CHAR16 Path[22];
    EFI_DEVICE_PATH_PROTOCOL End;
} BootFilePath;

static BootFilePath gBootFilePath = {
    .FileNode = {
        .Type = 0x04,
        .SubType = 0x04,
        .Length = {
            sizeof(EFI_DEVICE_PATH_PROTOCOL) + sizeof(CHAR16) * 22,
            0
        }
    },
    .Path = {
        '\\','E','F','I','\\',
        'B','O','O','T','\\',
        'B','O','O','T','X',
        '6','4','.','E','F',
        'I',0
    },
    .End = {
        .Type    = END_DEVICE_PATH_TYPE,
        .SubType = END_ENTIRE_DEVICE_PATH_SUBTYPE,
        .Length  = {4, 0}
    }
};