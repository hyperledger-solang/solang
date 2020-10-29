use parity_wasm;
use parity_wasm::builder;
use parity_wasm::elements::{InitExpr, Instruction, Module};
use std::fs::File;
use std::io::Read;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;
use Target;

pub fn link(input: &[u8], name: &str, target: Target) -> Vec<u8> {
    if target == Target::Generic {
        // Cannot link generic object
        return input.to_vec();
    }

    let dir = tempdir().expect("failed to create temp directory for linking");

    let object_filename = dir.path().join(&format!("{}.o", name));
    let res_filename = dir.path().join(&format!("{}.wasm", name));

    let mut objectfile =
        File::create(object_filename.clone()).expect("failed to create object file");

    objectfile
        .write_all(input)
        .expect("failed to write object file to temp file");

    let mut command_line =
        String::from("wasm-ld -O3 --no-entry --allow-undefined --gc-sections --global-base=0");

    match target {
        Target::Ewasm => {
            command_line.push_str(" --export main");
        }
        Target::Sabre => {
            command_line.push_str(" --export entrypoint");
        }
        Target::Substrate => {
            command_line.push_str(" --export deploy --export call");
            command_line.push_str(" --import-memory --initial-memory=1048576 --max-memory=1048576");
        }
        _ => unreachable!(),
    }

    command_line.push_str(&format!(
        " {} -o {}",
        object_filename
            .to_str()
            .expect("temp path should be unicode"),
        res_filename.to_str().expect("temp path should be unicode")
    ));

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

    let mut module: Module =
        parity_wasm::deserialize_buffer(&output).expect("cannot deserialize llvm wasm");

    {
        let imports = module.import_section_mut().unwrap().entries_mut();
        let mut ind = 0;

        while ind < imports.len() {
            match target {
                Target::Ewasm => {
                    let module_name = if imports[ind].field().starts_with("print") {
                        "debug"
                    } else {
                        "ethereum"
                    };

                    *imports[ind].module_mut() = module_name.to_owned();
                }
                Target::Substrate => {
                    if imports[ind].field().starts_with("seal") {
                        *imports[ind].module_mut() = "seal0".to_owned();
                    }
                }
                _ => (),
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
        *init_expr = InitExpr::new(vec![
            Instruction::I32Const(0x10000 as i32),
            Instruction::End,
        ]);
    }

    let linked = builder::module().with_module(module);

    parity_wasm::serialize(linked.build()).expect("cannot serialize linked wasm")
}
