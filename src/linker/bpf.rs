// Use the llvm lld linker to create our solana shared object
// This requires an lld with the solana bpf patches.

// Note it is possible to convert the single ELF reloc file to a dynamic file ourselves;
// this would require creating a dynamic program header in the elf file

// Using the llvm linker does give some possibilities around linking non-Solidity files
// and doing link time optimizations

use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;
pub fn link(input: &[u8], name: &str) -> Vec<u8> {
    let dir = tempdir().expect("failed to create temp directory for linking");

    let object_filename = dir.path().join(&format!("{}.o", name));
    let ldscript_filename = dir.path().join("bpf.ld");
    let res_filename = dir.path().join(&format!("{}.so", name));

    let mut objectfile =
        File::create(object_filename.clone()).expect("failed to create object file");

    objectfile
        .write_all(input)
        .expect("failed to write object file to temp file");

    let mut linker_script =
        File::create(ldscript_filename.clone()).expect("failed to create object file");

    linker_script
        .write_all(
            br##"PHDRS
{
    text PT_LOAD  ;
    rodata PT_LOAD ;
    dynamic PT_DYNAMIC ;
}

SECTIONS
{
    . = SIZEOF_HEADERS;
    .text : { *(.text) } :text
    .rodata : { *(.rodata) } :rodata
    .dynamic : { *(.dynamic) } :dynamic
    .dynsym : { *(.dynsym) } :dynamic
    .dynstr : { *(.dynstr) } :dynamic
    .gnu.hash : { *(.gnu.hash) } :dynamic
    .rel.dyn : { *(.rel.dyn) } :dynamic
    .hash : { *(.hash) } :dynamic
}"##,
        )
        .expect("failed to write linker script to temp file");

    let command_line = format!(
        "ld.lld  -z notext -shared --Bdynamic {} --entry entrypoint {} -o {}",
        ldscript_filename
            .to_str()
            .expect("temp path should be unicode"),
        object_filename
            .to_str()
            .expect("temp path should be unicode"),
        res_filename.to_str().expect("temp path should be unicode"),
    );

    let status = if cfg!(target_os = "windows") {
        Command::new("cmd")
            .args(&["/C", &command_line])
            .status()
            .expect("linker failed")
    } else {
        Command::new("sh")
            .arg("-c")
            .arg(command_line)
            .status()
            .expect("linker failed")
    };

    if !status.success() {
        panic!("linker failed");
    }

    let mut output = Vec::new();
    // read the whole file
    let mut outputfile = File::open(res_filename).expect("output file should exist");

    outputfile
        .read_to_end(&mut output)
        .expect("failed to read output file");

    output
}
