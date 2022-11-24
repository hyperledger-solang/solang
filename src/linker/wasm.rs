// SPDX-License-Identifier: Apache-2.0

use parity_wasm::builder;
use parity_wasm::elements::{InitExpr, Instruction, Module};
use std::ffi::CString;
use std::fs::File;
use std::io::Read;
use std::io::Write;
use tempfile::tempdir;

pub fn link(input: &[u8], name: &str) -> Vec<u8> {
    let dir = tempdir().expect("failed to create temp directory for linking");

    let object_filename = dir.path().join(format!("{}.o", name));
    let res_filename = dir.path().join(format!("{}.wasm", name));

    let mut objectfile =
        File::create(object_filename.clone()).expect("failed to create object file");

    objectfile
        .write_all(input)
        .expect("failed to write object file to temp file");

    let mut command_line = vec![
        CString::new("-O3").unwrap(),
        CString::new("--no-entry").unwrap(),
        CString::new("--allow-undefined").unwrap(),
        CString::new("--gc-sections").unwrap(),
        CString::new("--global-base=0").unwrap(),
    ];

    command_line.push(CString::new("--export").unwrap());
    command_line.push(CString::new("deploy").unwrap());
    command_line.push(CString::new("--export").unwrap());
    command_line.push(CString::new("call").unwrap());

    command_line.push(CString::new("--import-memory").unwrap());
    command_line.push(CString::new("--initial-memory=1048576").unwrap());
    command_line.push(CString::new("--max-memory=1048576").unwrap());

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

    assert!(!super::wasm_linker(&command_line), "linker failed");

    let mut output = Vec::new();
    // read the whole file
    let mut outputfile = File::open(res_filename).expect("output file should exist");

    outputfile
        .read_to_end(&mut output)
        .expect("failed to read output file");

    let mut module: Module =
        parity_wasm::deserialize_buffer(&output).expect("cannot deserialize llvm wasm");

    {
        let imports = module.import_section_mut().unwrap().entries_mut();
        let mut ind = 0;

        while ind < imports.len() {
            if imports[ind].field().starts_with("seal") {
                let module_name = match imports[ind].field() {
                    "seal_set_storage" => "seal2",
                    "seal_clear_storage"
                    | "seal_contains_storage"
                    | "seal_get_storage"
                    | "seal_instantiate"
                    | "seal_terminate"
                    | "seal_random"
                    | "seal_call" => "seal1",
                    _ => "seal0",
                };
                *imports[ind].module_mut() = module_name.to_owned();
            }

            ind += 1;
        }
    }

    // remove empty initializers
    if let Some(data_section) = module.data_section_mut() {
        let entries = data_section.entries_mut();
        let mut index = 0;

        while index < entries.len() {
            if entries[index].value().iter().all(|b| *b == 0) {
                entries.remove(index);
            } else {
                index += 1;
            }
        }
    }

    // set stack pointer to 64k (there is only one global)
    for global in module.global_section_mut().unwrap().entries_mut() {
        let init_expr = global.init_expr_mut();
        *init_expr = InitExpr::new(vec![Instruction::I32Const(0x10000), Instruction::End]);
    }

    let linked = builder::module().with_module(module);

    parity_wasm::serialize(linked.build()).expect("cannot serialize linked wasm")
}
