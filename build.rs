use std::{fs, path::Path, process::Command};
use walkdir::WalkDir;

const ASM: &str = "nasm";
const CC: &str = "i686-elf-gcc";
const CC64: &str = "x86_64-linux-gnu-gcc";
const LD: &str = "ld";
const OBJ: &str = "objcopy";

const FIRMWARE_PATH: &str = "guest/firmware";
const DISK_FILE: &str = "guest/disk.bin";

fn build_firmware() {
    fs::create_dir_all(FIRMWARE_PATH.to_owned() + "/build").unwrap();

    let asm_entry_input = FIRMWARE_PATH.to_owned() + "/assembly/entry.asm";
    let asm_entry_output = FIRMWARE_PATH.to_owned() + "/build/entry.o";

    let status = Command::new(ASM)
        .args(["-f", "elf32", asm_entry_input.as_str(), "-o", asm_entry_output.as_str()])
        .status()
        .expect("failed to run nasm");

    if !status.success() {
        panic!("nasm failed to assemble firmware entry stub");
    }

    let asm_idt_input = FIRMWARE_PATH.to_owned() + "/assembly/idt_handlers.asm";
    let asm_idt_output = FIRMWARE_PATH.to_owned() + "/build/idt_handlers.o";

    let status = Command::new(ASM)
        .args(["-f", "elf64", asm_idt_input.as_str(), "-o", asm_idt_output.as_str()])
        .status()
        .expect("failed to run nasm");

    if !status.success() {
        panic!("nasm failed to assemble firmware entry stub");
    }

    let cc_input = FIRMWARE_PATH.to_owned() + "/main.c";
    let cc_output = FIRMWARE_PATH.to_owned() + "/build/main.o";

    let status = Command::new(CC)
        .args(["-m32", "-ffreestanding", "-fno-stack-protector", "-nostdlib", "-isystem", "/usr/lib/gcc/x86_64-linux-gnu/13/include", "-O2", "-c", cc_input.as_str(), "-o", cc_output.as_str()])
        .status()
        .expect("failed to run gcc");

    if !status.success() {
        panic!("gcc failed to compile firmware c_main");
    }

    let cc64_input = FIRMWARE_PATH.to_owned() + "/main64.c";
    let cc64_output = FIRMWARE_PATH.to_owned() + "/build/main64.o";

    let status = Command::new(CC64)
        .args([
            "-m64", "-ffreestanding", "-fno-stack-protector",
            "-mno-red-zone", "-mcmodel=kernel", "-fno-pic", "-fno-pie",
            "-nostdlib",
            "-isystem", "/usr/lib/gcc/x86_64-linux-gnu/13/include",
            "-O2", "-c",
            cc64_input.as_str(), "-o", cc64_output.as_str()
        ])
        .status().expect("failed to run gcc64");
    if !status.success() { panic!("gcc64 failed"); }

    let ld64_script  = FIRMWARE_PATH.to_owned() + "/linker64.ld";
    let ld64_elf     = FIRMWARE_PATH.to_owned() + "/build/main64.elf";
    let ld64_bin     = FIRMWARE_PATH.to_owned() + "/build/main64.bin";

    let status = Command::new("ld")
        .args([
            "-T", ld64_script.as_str(),
            "-o", ld64_elf.as_str(),
            asm_idt_output.as_str(),
            cc64_output.as_str(),
        ])
        .status().expect("failed to run ld64");
    if !status.success() { panic!("ld64 failed"); }

    let status = Command::new(OBJ)
        .args(["-O", "binary", ld64_elf.as_str(), ld64_bin.as_str()])
        .status().expect("failed to run objcopy for main64");
    if !status.success() { panic!("objcopy main64 failed"); }

    let ld_output = FIRMWARE_PATH.to_owned() + "/build/out.elf";
    let ld_script = FIRMWARE_PATH.to_owned() + "/linker.ld";

    let status = Command::new(LD)
        .args([
            "-m", "elf_i386",
            "-T", ld_script.as_str(),
            "-o", ld_output.as_str(),
            asm_entry_output.as_str(),
            cc_output.as_str(),
        ])
        .status()
        .expect("failed to run ld");

    if !status.success() {
        panic!("ld failed to link firmware");
    }

    let obj_output = FIRMWARE_PATH.to_owned() + "/build/out.bin";

    let status = Command::new(OBJ)
        .args(["-O", "binary", ld_output.as_str(), obj_output.as_str()])
        .status()
        .expect("failed to run objcopy");

    if !status.success() {
        panic!("objcopy failed to create flat binary");
    }
}

fn ensure_disk_file() {
    let path = Path::new(DISK_FILE);

    if !path.is_file() {
        let status = Command::new("dd")
            .args(["if=/dev/random", format!("of={}", DISK_FILE).as_str(), "bs=1M", "count=64"])
            .status()
            .expect("failed to run nasm");

        if !status.success() {
            panic!("DD could not make disk");
        }
    }
}

fn build() {
    build_firmware();
    ensure_disk_file();

    for entry in WalkDir::new("guest/test").into_iter().filter_map(Result::ok) {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("asm") {
            continue;
        }

        println!("Compiling {}", path.display());

        let input = path.to_str().unwrap();
        let output = path.with_extension("bin");
        let output = output.to_str().unwrap();

        let status = Command::new(ASM)
            .args(["-f", "bin", input, "-o", output])
            .status()
            .expect("failed to run nasm");

        if !status.success() {
            panic!("nasm failed on {}", input);
        }
    }
}

fn main() {
    build(); // MUST exit immediately
}