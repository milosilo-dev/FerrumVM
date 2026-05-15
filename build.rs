use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

const ASM: &str = "nasm";
const ASL: &str = "iasl";
const CC32: &str = "i686-elf-gcc";
const CC64: &str = "x86_64-linux-gnu-gcc";
const LD: &str = "ld";
const OBJCOPY: &str = "objcopy";

const FIRMWARE_DIR: &str = "guest/firmware";
const BUILD_DIR: &str = "guest/firmware/build";
const ACPI_DIR: &str = "acpi";
const DISK_FILE: &str = "guest/disk.bin";

fn run(cmd: &str, args: &[&str], error: &str) {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("{error}: {e}"));

    if !status.success() {
        panic!("{error}");
    }
}

fn build_acpi() {
    let dsdt = format!("{ACPI_DIR}/DSDT.dsl");

    run(
        ASL,
        &["-tc", &dsdt],
        "failed to compile DSDT",
    );
}

fn assemble(input: &str, output: &str, format: &str) {
    run(
        ASM,
        &["-f", format, input, "-o", output],
        &format!("failed to assemble {input}"),
    );
}

fn compile_c32(input: &str, output: &str) {
    run(
        CC32,
        &[
            "-m32",
            "-ffreestanding",
            "-fno-stack-protector",
            "-nostdlib",
            "-isystem",
            "/usr/lib/gcc/x86_64-linux-gnu/13/include",
            "-O2",
            "-c",
            input,
            "-o",
            output,
        ],
        &format!("failed to compile {input}"),
    );
}

fn compile_c64(input: &str, output: &str) {
    run(
        CC64,
        &[
            "-m64",
            "-ffreestanding",
            "-fno-stack-protector",
            "-mno-red-zone",
            "-mcmodel=kernel",
            "-fno-pic",
            "-fno-pie",
            "-nostdlib",
            "-isystem",
            "/usr/lib/gcc/x86_64-linux-gnu/13/include",
            "-O2",
            "-c",
            input,
            "-o",
            output,
        ],
        &format!("failed to compile {input}"),
    );
}

fn link(
    output: &str,
    linker_script: &str,
    inputs: &[&str],
    extra_args: &[&str],
) {
    let mut args = Vec::new();

    args.extend_from_slice(extra_args);
    args.extend_from_slice(&["-T", linker_script, "-o", output]);
    args.extend_from_slice(inputs);

    run(LD, &args, "linking failed");
}

fn objcopy_binary(input: &str, output: &str) {
    run(
        OBJCOPY,
        &["-O", "binary", input, output],
        &format!("failed to objcopy {input}"),
    );
}

fn build_firmware() {
    fs::create_dir_all(BUILD_DIR).unwrap();

    build_acpi();

    // ===== Assembly =====

    let entry32_o = format!("{BUILD_DIR}/entry.o");
    assemble(
        &format!("{FIRMWARE_DIR}/assembly/entry.asm"),
        &entry32_o,
        "elf32",
    );

    let idt_o = format!("{BUILD_DIR}/idt_handlers.o");
    assemble(
        &format!("{FIRMWARE_DIR}/assembly/idt_handlers.asm"),
        &idt_o,
        "elf64",
    );

    let entry64_o = format!("{BUILD_DIR}/entry64.o");
    assemble(
        &format!("{FIRMWARE_DIR}/assembly/entry64.asm"),
        &entry64_o,
        "elf64",
    );

    // ===== C =====

    let main32_o = format!("{BUILD_DIR}/main.o");
    compile_c32(
        &format!("{FIRMWARE_DIR}/main.c"),
        &main32_o,
    );

    let main64_o = format!("{BUILD_DIR}/main64.o");
    compile_c64(
        &format!("{FIRMWARE_DIR}/main64.c"),
        &main64_o,
    );

    // ===== 64-bit firmware =====

    let main64_elf = format!("{BUILD_DIR}/main64.elf");
    let main64_bin = format!("{BUILD_DIR}/main64.bin");

    link(
        &main64_elf,
        &format!("{FIRMWARE_DIR}/linkerscript/linker64.ld"),
        &[&idt_o, &entry64_o, &main64_o],
        &[],
    );

    objcopy_binary(&main64_elf, &main64_bin);

    // ===== 32-bit firmware =====

    let firmware_elf = format!("{BUILD_DIR}/out.elf");
    let firmware_bin = format!("{BUILD_DIR}/out.bin");

    link(
        &firmware_elf,
        &format!("{FIRMWARE_DIR}/linkerscript/linker.ld"),
        &[&entry32_o, &main32_o],
        &["-m", "elf_i386"],
    );

    objcopy_binary(&firmware_elf, &firmware_bin);
}

fn ensure_disk_file() {
    if Path::new(DISK_FILE).exists() {
        return;
    }

    run(
        "dd",
        &[
            "if=/dev/random",
            &format!("of={DISK_FILE}"),
            "bs=1M",
            "count=64",
        ],
        "failed to create disk image",
    );
}

fn build_tests() {
    for entry in WalkDir::new("guest/test")
        .into_iter()
        .filter_map(Result::ok)
    {
        let path = entry.path();

        if !path.is_file() {
            continue;
        }

        if path.extension().and_then(|e| e.to_str()) != Some("asm") {
            continue;
        }

        compile_test(path);
    }
}

fn compile_test(path: &Path) {
    println!("Compiling {}", path.display());

    let output: PathBuf = path.with_extension("bin");

    run(
        ASM,
        &[
            "-f",
            "bin",
            path.to_str().unwrap(),
            "-o",
            output.to_str().unwrap(),
        ],
        &format!("failed to assemble {}", path.display()),
    );
}

fn main() {
    build_firmware();
    ensure_disk_file();
    build_tests();
}