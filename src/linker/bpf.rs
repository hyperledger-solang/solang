// Use the llvm lld linker to create our solana shared object
// This requires an lld with the solana bpf patches.

// Note it is possible to convert the single ELF reloc file to a dynamic file ourselves;
// this would require creating a dynamic program header in the elf file

// Using the llvm linker does give some possibilities around linking non-Solidity files
// and doing link time optimizations

use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use tempfile::tempdir;

pub fn link(input: &[u8], name: &str) -> Vec<u8> {
    let dir = tempdir().expect("failed to create temp directory for linking");

    let object_filename = dir.path().join(&format!("{}.o", name));
    let res_filename = dir.path().join(&format!("{}.so", name));
    let linker_script_filename = dir.path().join("linker.ld");

    let mut objectfile =
        File::create(object_filename.clone()).expect("failed to create object file");

    objectfile
        .write_all(input)
        .expect("failed to write object file to temp file");

    let mut linker_script =
        File::create(linker_script_filename.clone()).expect("failed to create linker script");

    linker_script
        .write_all(
            br##"
ENTRY(entrypoint)

PHDRS
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

    let mut command_line = Vec::new();

    command_line.push(CString::new("-z").unwrap());
    command_line.push(CString::new("notext").unwrap());
    command_line.push(CString::new("-shared").unwrap());
    command_line.push(CString::new("--Bdynamic").unwrap());
    command_line.push(
        CString::new(
            linker_script_filename
                .to_str()
                .expect("temp path should be unicode"),
        )
        .unwrap(),
    );
    command_line.push(
        CString::new(
            object_filename
                .to_str()
                .expect("temp path should be unicode"),
        )
        .unwrap(),
    );
    command_line.push(CString::new("-o").unwrap());
    command_line
        .push(CString::new(res_filename.to_str().expect("temp path should be unicode")).unwrap());

    if super::elf_linker(&command_line) {
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
