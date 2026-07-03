use std::{
    env, fs,
    path::{Path, PathBuf},
    process::Command,
};

use walkdir::WalkDir;

const FIRMWARE_DIR: &str = "guest/firmware";
const BUILD_DIR: &str = "guest/firmware/build";
const ACPI_DIR: &str = "acpi";

fn tool(env_var: &str, default: &str) -> String {
    env::var(env_var).unwrap_or_else(|_| default.to_owned())
}

fn run(cmd: &str, args: &[&str], error: &str) {
    let status = Command::new(cmd)
        .args(args)
        .status()
        .unwrap_or_else(|e| panic!("{error}: {e}"));

    if !status.success() {
        panic!("{error}");
    }
}

fn build_acpi(asl: &str) {
    let dsdt = format!("{ACPI_DIR}/DSDT.dsl");
    run(asl, &["-tc", &dsdt], "failed to compile DSDT");
}

fn assemble(asm: &str, input: &str, output: &str, format: &str) {
    run(
        asm,
        &["-f", format, input, "-o", output],
        &format!("failed to assemble {input}"),
    );
}

fn compile_c32(cc32: &str, input: &str, output: &str) {
    run(
        cc32,
        &[
            "-m32",
            "-ffreestanding",
            "-fno-stack-protector",
            "-nostdlib",
            "-O2",
            "-c",
            input,
            "-o",
            output,
        ],
        &format!("failed to compile {input}"),
    );
}

fn compile_c64(cc64: &str, input: &str, output: &str) {
    run(
        cc64,
        &[
            "-m64",
            "-ffreestanding",
            "-fno-stack-protector",
            "-mno-red-zone",
            "-mcmodel=kernel",
            "-fno-pic",
            "-fno-pie",
            "-nostdlib",
            "-O2",
            "-c",
            input,
            "-o",
            output,
        ],
        &format!("failed to compile {input}"),
    );
}

fn link(ld: &str, output: &str, linker_script: &str, inputs: &[&str], extra_args: &[&str]) {
    let mut args = Vec::new();

    args.extend_from_slice(extra_args);
    args.extend_from_slice(&["-T", linker_script, "-o", output]);
    args.extend_from_slice(inputs);

    run(ld, &args, "linking failed");
}

fn objcopy_binary(objcopy: &str, input: &str, output: &str) {
    run(
        objcopy,
        &["-O", "binary", input, output],
        &format!("failed to objcopy {input}"),
    );
}

fn build_firmware(asm: &str, asl: &str, cc32: &str, cc64: &str, ld: &str, objcopy: &str) {
    fs::create_dir_all(BUILD_DIR).unwrap();

    build_acpi(asl);

    // ===== Assembly =====

    let entry32_o = format!("{BUILD_DIR}/entry.o");
    assemble(
        asm,
        &format!("{FIRMWARE_DIR}/assembly/entry.asm"),
        &entry32_o,
        "elf32",
    );

    let idt_o = format!("{BUILD_DIR}/idt_handlers.o");
    assemble(
        asm,
        &format!("{FIRMWARE_DIR}/assembly/idt_handlers.asm"),
        &idt_o,
        "elf64",
    );

    let entry64_o = format!("{BUILD_DIR}/entry64.o");
    assemble(
        asm,
        &format!("{FIRMWARE_DIR}/assembly/entry64.asm"),
        &entry64_o,
        "elf64",
    );

    // ===== C =====

    let main32_o = format!("{BUILD_DIR}/main.o");
    compile_c32(cc32, &format!("{FIRMWARE_DIR}/main.c"), &main32_o);

    let main64_o = format!("{BUILD_DIR}/main64.o");
    compile_c64(cc64, &format!("{FIRMWARE_DIR}/main64.c"), &main64_o);

    // ===== 64-bit firmware =====

    let main64_elf = format!("{BUILD_DIR}/main64.elf");
    let main64_bin = format!("{BUILD_DIR}/main64.bin");

    link(
        ld,
        &main64_elf,
        &format!("{FIRMWARE_DIR}/linkerscript/linker64.ld"),
        &[&idt_o, &entry64_o, &main64_o],
        &[],
    );

    objcopy_binary(objcopy, &main64_elf, &main64_bin);

    // ===== 32-bit firmware =====

    let firmware_elf = format!("{BUILD_DIR}/out.elf");
    let firmware_bin = format!("{BUILD_DIR}/out.bin");

    link(
        ld,
        &firmware_elf,
        &format!("{FIRMWARE_DIR}/linkerscript/linker.ld"),
        &[&entry32_o, &main32_o],
        &["-m", "elf_i386"],
    );

    objcopy_binary(objcopy, &firmware_elf, &firmware_bin);
}

fn build_tests(asm: &str) {
    for entry in WalkDir::new("guest/tests")
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

        compile_test(asm, path);
    }
}

fn compile_test(asm: &str, path: &Path) {
    println!("Compiling {}", path.display());

    let output: PathBuf = path.with_extension("bin");

    run(
        asm,
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
    let asm = tool("FERRUM_ASM", "nasm");
    let asl = tool("FERRUM_ASL", "iasl");
    let cc32 = tool("FERRUM_CC32", "i686-elf-gcc");
    let cc64 = tool("FERRUM_CC64", "x86_64-linux-gnu-gcc");
    let ld = tool("FERRUM_LD", "ld");
    let objcopy = tool("FERRUM_OBJCOPY", "objcopy");

    build_firmware(&asm, &asl, &cc32, &cc64, &ld, &objcopy);
    build_tests(&asm);
}
