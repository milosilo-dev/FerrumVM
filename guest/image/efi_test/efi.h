typedef void* EFI_HANDLE;
typedef unsigned long long EFI_STATUS;

typedef struct {
    char _buf[52];
    unsigned short* ConOut;
} EFI_SYSTEM_TABLE;

#define EFI_SUCCESS 0