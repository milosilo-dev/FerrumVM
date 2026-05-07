#pragma once
#include <stdint.h>

typedef struct {
  uint16_t    e_magic;
  uint16_t    e_cblp;
  uint16_t    e_cp;
  uint16_t    e_crlc;
  uint16_t    e_cparhdr;
  uint16_t    e_minalloc;
  uint16_t    e_maxalloc;
  uint16_t    e_ss;
  uint16_t    e_sp;
  uint16_t    e_csum;
  uint16_t    e_ip;
  uint16_t    e_cs;
  uint16_t    e_lfarlc;
  uint16_t    e_ovno;
  uint16_t    e_res[4];
  uint16_t    e_oemid;
  uint16_t    e_oeminfo;
  uint16_t    e_res2[10];
  uint32_t    e_lfanew;
} EFI_IMAGE_DOS_HEADER;

#define IMAGE_DOS_SIGNATURE        0x5A4D      // "MZ"
#define IMAGE_NT_SIGNATURE         0x00004550  // "PE\0\0"

#define IMAGE_FILE_MACHINE_AMD64   0x8664
#define IMAGE_FILE_EXECUTABLE_IMAGE     0x0002
#define IMAGE_SUBSYSTEM_EFI_APPLICATION 10
#define IMAGE_NUMBEROF_DIRECTORY_ENTRIES 16

typedef struct {
    uint16_t Machine;
    uint16_t NumberOfSections;
    uint32_t TimeDateStamp;
    uint32_t PointerToSymbolTable;
    uint32_t NumberOfSymbols;
    uint16_t SizeOfOptionalHeader;
    uint16_t Characteristics;
} IMAGE_FILE_HEADER;

typedef struct {
    uint32_t VirtualAddress;
    uint32_t Size;
} IMAGE_DATA_DIRECTORY;

typedef struct {
    uint16_t Magic;                         // 0x020B
    uint8_t  MajorLinkerVersion;
    uint8_t  MinorLinkerVersion;
    uint32_t SizeOfCode;
    uint32_t SizeOfInitializedData;
    uint32_t SizeOfUninitializedData;
    uint32_t AddressOfEntryPoint;
    uint32_t BaseOfCode;
    uint64_t ImageBase;
    uint32_t SectionAlignment;
    uint32_t FileAlignment;
    uint16_t MajorOperatingSystemVersion;
    uint16_t MinorOperatingSystemVersion;
    uint16_t MajorImageVersion;
    uint16_t MinorImageVersion;
    uint16_t MajorSubsystemVersion;
    uint16_t MinorSubsystemVersion;
    uint32_t Win32VersionValue;
    uint32_t SizeOfImage;
    uint32_t SizeOfHeaders;
    uint32_t CheckSum;
    uint16_t Subsystem;
    uint16_t DllCharacteristics;
    uint64_t SizeOfStackReserve;
    uint64_t SizeOfStackCommit;
    uint64_t SizeOfHeapReserve;
    uint64_t SizeOfHeapCommit;
    uint32_t LoaderFlags;
    uint32_t NumberOfRvaAndSizes;
    IMAGE_DATA_DIRECTORY DataDirectory[IMAGE_NUMBEROF_DIRECTORY_ENTRIES];
} IMAGE_OPTIONAL_HEADER64;

typedef struct {
    uint32_t              Signature;
    IMAGE_FILE_HEADER     FileHeader;
    IMAGE_OPTIONAL_HEADER64 OptionalHeader;
} IMAGE_NT_HEADERS64;