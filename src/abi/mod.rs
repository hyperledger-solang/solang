// SPDX-License-Identifier: Apache-2.0

use std::collections::HashSet;

use solang_parser::pt;

use crate::sema::ast::Namespace;
use crate::Target;

pub mod ethereum;
pub mod substrate;

pub fn generate_abi(
    contract_no: usize,
    ns: &Namespace,
    code: &[u8],
    verbose: bool,
) -> (String, &'static str) {
    match ns.target {
        Target::Substrate { .. } => {
            if verbose {
                eprintln!(
                    "info: Generating Substrate ABI for contract {}",
                    ns.contracts[contract_no].name
                );
            }

            let abi = substrate::metadata(contract_no, code, ns);

            (serde_json::to_string_pretty(&abi).unwrap(), "contract")
        }
        _ => {
            if verbose {
                eprintln!(
                    "info: Generating Ethereum ABI for contract {}",
                    ns.contracts[contract_no].name
                );
            }

            let abi = ethereum::gen_abi(contract_no, ns);

            (serde_json::to_string(&abi).unwrap(), "abi")
        }
    }
}

/// Returns a set of all non-unique public function names in a given contract.
/// These names should not be used in the metadata. Instead, the mangled versions should be used.
pub(super) fn non_unique_function_names(contract_no: usize, ns: &Namespace) -> HashSet<&String> {
    let mut names = HashSet::new();
    ns.contracts[contract_no]
        .all_functions
        .keys()
        .map(|f| &ns.functions[*f])
        .filter(|f| f.is_public())
        .filter(|f| f.ty == pt::FunctionTy::Function || f.ty == pt::FunctionTy::Constructor)
        .filter(|f| !names.insert(&f.name))
        .map(|f| &f.name)
        .collect()
}
