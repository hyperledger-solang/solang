// SPDX-License-Identifier: Apache-2.0

use crate::{
    borsh_encoding::{visit_mut, VisitorMut},
    AccountMeta, BorshToken, Pubkey, VirtualMachineBuilder,
};
use anchor_syn::idl::types::IdlAccountItem;
use once_cell::sync::Lazy;
use rayon::iter::ParallelIterator;
use rayon::prelude::IntoParallelIterator;
use serde::Deserialize;
use solang::codegen::Options;
use std::{
    env::var,
    fs::{read_dir, read_to_string, File},
    io::BufReader,
    path::Path,
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
        run_test(&calls.join(testname).with_extension("json"));
    } else {
        let tests = read_dir(calls)
            .unwrap()
            .map(|entry| entry.unwrap().path())
            .collect::<Vec<_>>();
        tests.into_par_iter().for_each(|path| run_test(&path));
        //tests.iter().for_each(|path| run_test(path));
    }
}

fn run_test(path: &Path) {
    let file_stem = path.file_stem().unwrap();

    // Known problematic test.
    if file_stem == "b6339ad75e9175a6bf332a2881001b6c928734e2" {
        return;
    }

    println!("testcase: {:?}", file_stem);

    let file =
        File::open(path).unwrap_or_else(|error| panic!("failed to open {path:?}: {error:?}"));
    let reader = BufReader::new(file);
    let calls: Calls = serde_json::from_reader(reader).unwrap();

    let path = Path::new("tests/optimization_testcases/programs")
        .join(file_stem)
        .with_extension("sol");
    let program = read_to_string(path).unwrap();

    run_test_with_opts(
        &program,
        &calls,
        [Options::default(), NO_OPTIMIZATIONS.clone()],
    );
}

fn run_test_with_opts<T: IntoIterator<Item = Options>>(program: &str, calls: &Calls, opts: T) {
    let mut results_prev: Option<Vec<Result<Option<BorshToken>, u64>>> = None;

    for (i, opts) in opts.into_iter().enumerate() {
        println!("iteration: {i}");

        let mut results_curr = Vec::new();

        let mut vm = VirtualMachineBuilder::new(program).opts(opts).build();

        let data_account = vm.initialize_data_account();

        results_curr.push(
            vm.function("new")
                .arguments(&calls.constructor)
                .accounts(vec![("dataAccount", data_account)])
                .call_with_error_code(),
        );

        let program_id = vm.stack[0].id;
        for (name, args) in &calls.function {
            let needs_account = vm.stack[0]
                .idl
                .as_ref()
                .unwrap()
                .instructions
                .iter()
                .find(|instr| &instr.name == name)
                .unwrap()
                .accounts
                .iter()
                .any(|acc| match acc {
                    IdlAccountItem::IdlAccount(account) => account.name == "dataAccount",
                    IdlAccountItem::IdlAccounts(_) => false,
                });

            results_curr.push(if needs_account {
                vm.function(name)
                    .arguments(args)
                    .accounts(vec![("dataAccount", data_account)])
                    .call_with_error_code()
            } else {
                vm.function(name)
                    .arguments(args)
                    .remaining_accounts(&[AccountMeta {
                        pubkey: Pubkey(program_id),
                        is_signer: false,
                        is_writable: false,
                    }])
                    .call_with_error_code()
            });
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

// If `AddressEraser` were not used above, one would see failures with programs that return
// addresses, e.g.:
//   thread 'solana_tests::optimizations::optimizations' panicked at 'assertion failed: `(left == right)`
//     left: `[Ok(Some(Address([/* one sequence of random bytes */])))]`,
//    right: `[Ok(Some(Address([/* another sequence of random bytes */])))]`', tests/solana_tests/optimizations.rs:105:13
// d55b66a2225baa2bd6cd3641fff28de6fdf9b30e.sol is an example of such a program.
struct AddressEraser;

impl VisitorMut for AddressEraser {
    fn visit_address(&mut self, a: &mut [u8; 32]) {
        a.copy_from_slice(&[0u8; 32]);
    }
}
