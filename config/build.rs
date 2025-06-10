#![no_std]

use std::env;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out_dir = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Bağlayıcı betiği ve bellek haritası
    create_and_rerun(out_dir.join("linker.ld"), include_bytes!("linker.ld"));
    create_and_rerun(out_dir.join("memory.x"), include_bytes!("memory.x"));
    println!("cargo:rustc-link-search={}", out_dir.display());

    // Assembly dosyalarını derle
    compile_assembly_files();

    // Rust dosyalarını derle ve bağla
    compile_rust_files();
}

fn create_and_rerun(path: PathBuf, content: &[u8]) {
    fs::File::create(&path).unwrap().write_all(content).unwrap();
    println!("cargo:rerun-if-changed={}", path.display());
}

fn compile_assembly_files() {
    let assembly_files = [
        "srcboot_arm.S", "srcboot_elbrus.S", "srcboot_loongarch.S",
        "srcboot_mips.S", "srcboot_openrisc.S", "srcboot_powerpc.S",
        "srcboot_sparc.S", "srcboot_x86.S", "srcboot_riscv.S",
        "srcentry_arm.asm", "srcentry_elbrus.asm", "srcentry_loongarch.asm",
        "srcentry_mips.asm", "srcentry_openrisc.asm", "srcentry_powerpc.asm",
        "srcentry_sparc.asm", "srcentry_x86.asm", "srcentry_riscv.asm",
    ];

    for file in &assembly_files {
        println!("cargo:rerun-if-changed={}", file);
    }

    let mut build = cc::Build::new();
    for file in &assembly_files {
        build.file(file);
    }
    build.compile("boot_entry");
}

fn compile_rust_files() {
    println!("cargo:rustc-link-lib=core");
    println!("cargo:rustc-link-arg=-Tlinker.ld");
    println!("cargo:rustc-flags=-Z unstable-options --emit=obj");

    // Modül bağımlılıklarını ve derleme sırasını burada belirleyin
    let modules = [

    ];

    for module in &modules {
        println!("cargo:rerun-if-changed=src/{}.rs", module);
    }
}
