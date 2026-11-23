// SPDX-License-Identifier: Apache-2.0

//! Checks the shape of the riscv target's output; the contracts themselves
//! are run by the r55 repository's `solang_*` tests. Skipped without an `llc`
//! that has the RISC-V backend.

use inkwell::values::AnyValue;
use solang::codegen::{codegen, OptimizationLevel, Options};
use solang::file_resolver::FileResolver;
use solang::{compile, parse_and_resolve, Target};
use std::ffi::OsStr;
use std::fs;

const TESTCASES: &str = "tests/contract_testcases/riscv";

const R55_ELF: &[u8] = b"\xff\x7fELF";

fn have_llc() -> bool {
    match solang::emit::riscv::codegen::find_llc() {
        Ok(_) => true,
        Err(err) => {
            eprintln!("skipping riscv test: {err}");
            false
        }
    }
}

fn options() -> Options {
    Options {
        opt_level: OptimizationLevel::Default,
        ..Default::default()
    }
}

#[test]
fn testcases_compile_to_r55_deploy_code() {
    if !have_llc() {
        return;
    }

    for entry in fs::read_dir(TESTCASES).unwrap() {
        let path = entry.unwrap().path();
        if path.extension() != Some(OsStr::new("sol")) {
            continue;
        }

        let mut resolver = FileResolver::default();
        resolver.add_import_path(path.parent().unwrap());
        let file = path.file_name().unwrap();
        let (outputs, ns) = compile(
            file,
            &mut resolver,
            Target::Riscv,
            &options(),
            vec![],
            "0.0.1",
        );
        ns.print_diagnostics_in_plain(&resolver, false);
        assert!(
            !ns.diagnostics.any_errors(),
            "{} has errors",
            path.display()
        );
        assert!(!outputs.is_empty(), "{} has no contracts", path.display());

        for (code, name) in outputs {
            assert_eq!(&code[..5], R55_ELF, "{name}: starts with 0xFF + ELF magic");

            let elf = &code[1..];
            assert_eq!(elf[4], 2, "{name}: 64-bit ELF");
            let machine = u16::from_le_bytes([elf[0x12], elf[0x13]]);
            assert_eq!(machine, 243, "{name}: e_machine is RISC-V");
            let entry = u64::from_le_bytes(elf[0x18..0x20].try_into().unwrap());
            assert_eq!(entry, 0x8030_0000, "{name}: entry point");

            let runtime_at = elf
                .windows(R55_ELF.len())
                .position(|w| w == R55_ELF)
                .unwrap_or_else(|| panic!("{name}: the runtime image is embedded"));
            assert!(runtime_at > 0);
        }
    }
}

#[test]
fn deploy_image_initializes_constructs_and_returns_runtime() {
    if !have_llc() {
        return;
    }

    let mut resolver = FileResolver::default();
    resolver.add_import_path(&std::path::PathBuf::from(TESTCASES));
    let mut ns = parse_and_resolve(OsStr::new("constructor.sol"), &mut resolver, Target::Riscv);
    let opt = options();
    codegen(&mut ns, &opt);
    assert!(!ns.diagnostics.any_errors());

    let contract_no = ns
        .contracts
        .iter()
        .position(|c| c.id.name == "with_constructor")
        .unwrap();

    let context = inkwell::context::Context::create();
    let deploy = ns.contracts[contract_no].binary(&ns, &context, &opt, contract_no);

    let entry = deploy
        .module
        .get_function("solang_dispatch")
        .unwrap()
        .print_to_string()
        .to_string();

    let position = |callee: &str| {
        entry
            .find(&format!("@{callee}("))
            .unwrap_or_else(|| panic!("deploy solang_dispatch calls {callee}:\n{entry}"))
    };

    assert!(position("storage_initializer") < position("riscv_deploy_dispatch"));
    assert!(position("riscv_deploy_dispatch") < position("__sys_return"));

    let runtime = deploy.runtime.as_ref().expect("deploy image has a runtime");
    assert!(runtime.module.get_function("storage_initializer").is_none());
    assert!(runtime
        .module
        .get_function("riscv_deploy_dispatch")
        .is_none());
}
