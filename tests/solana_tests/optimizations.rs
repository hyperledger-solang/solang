use crate::{
    borsh_encoding::{visit_mut, VisitorMut},
    BorshToken, VirtualMachineBuilder,
};
use anchor_syn::idl::IdlInstruction;
use once_cell::sync::Lazy;
use serde::Deserialize;
use solang::codegen::Options;
use std::{
    env::var,
    fs::{read_dir, read_to_string, File},
    io::BufReader,
    path::{Path, PathBuf},
};

#[derive(Debug, Deserialize)]
struct Calls {
    constructor: Vec<BorshToken>,
    function: Vec<(String, Vec<BorshToken>)>,
}

static NO_OPTIMIZATIONS: Lazy<Options> = Lazy::new(|| Options {
    dead_storage: false,
    constant_folding: false,
    strength_reduce: false,
    vector_to_slice: false,
    common_subexpression_elimination: false,
    ..Default::default()
});

#[test]
fn optimizations() {
    let calls = Path::new("tests/optimization_testcases/calls");

    if let Ok(testname) = var("TESTNAME") {
        run_tests(std::iter::once(calls.join(testname).with_extension("json")));
    } else {
        run_tests(read_dir(calls).unwrap().map(|entry| entry.unwrap().path()));
    }
}

fn run_tests(iter: impl Iterator<Item = PathBuf>) {
    for path in iter {
        let file_stem = path.file_stem().unwrap();

        // Known problematic test.
        if file_stem == "a467d917a5158b613f94954d4a558a5cb61e6773" {
            continue;
        }

        dbg!(file_stem);

        let file = File::open(&path).unwrap();
        let reader = BufReader::new(file);
        let calls: Calls = serde_json::from_reader(reader).unwrap();

        let path = Path::new("tests/optimization_testcases/programs")
            .join(file_stem)
            .with_extension("sol");
        let program = read_to_string(path).unwrap();

        run_one_test(
            &program,
            &calls,
            [Options::default(), NO_OPTIMIZATIONS.clone()],
        );
    }
}

fn run_one_test(program: &str, calls: &Calls, opts: impl IntoIterator<Item = Options>) {
    let mut results_prev: Option<Vec<Result<Option<BorshToken>, u64>>> = None;

    for (i, opts) in opts.into_iter().enumerate() {
        dbg!(i);

        let mut results_curr = Vec::new();

        let mut vm = VirtualMachineBuilder::new(program).opts(opts).build();

        let data_account = vm.initialize_data_account();

        results_curr.push(
            vm.function("new")
                .arguments(&calls.constructor)
                .accounts(vec![("dataAccount", data_account)])
                .call_with_error_code(),
        );

        for (name, args) in &calls.function {
            if let Some(idl) = &vm.stack[0].idl {
                let mut idl = idl.clone();

                idl.instructions.push(IdlInstruction {
                    name: name.clone(),
                    docs: None,
                    accounts: vec![],
                    args: vec![],
                    returns: None,
                });

                vm.stack[0].idl = Some(idl);
            }

            results_curr.push(
                vm.function(name)
                    .arguments(args)
                    .accounts(vec![("dataAccount", data_account)])
                    .call_with_error_code(),
            );
        }

        for token in results_curr.iter_mut().flatten().flatten() {
            visit_mut(&mut AddressEraser, token);
        }

        if let Some(results_prev) = &results_prev {
            assert_eq!(results_prev, &results_curr);
        } else {
            results_prev = Some(results_curr);
        }
    }
}

struct AddressEraser;

impl VisitorMut for AddressEraser {
    fn visit_address(&mut self, a: &mut [u8; 32]) {
        a.copy_from_slice(&[0u8; 32]);
    }
}
