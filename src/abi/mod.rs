// SPDX-License-Identifier: Apache-2.0

use std::collections::{HashMap, HashSet};

use solang_parser::pt;

use crate::sema::ast::{self, Namespace};
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

/// Given a contract number, check for duplicated names in all public and external functions.
/// Returns a map of functions which should be mangled, together with their mangled names.
pub fn mangle(contract_no: usize, ns: &ast::Namespace) -> HashMap<usize, String> {
    let mut all_names = HashSet::new();
    ns.contracts[contract_no]
        .all_functions
        .keys()
        .map(|no| (*no, &ns.functions[*no]))
        .filter(|(_, function)| match (&function.visibility, function.ty) {
            (pt::Visibility::Public(_) | pt::Visibility::External(_), pt::FunctionTy::Function) => {
                !all_names.insert(&function.name)
            }
            _ => false,
        })
        .map(|(no, function)| (no, mangle_signature(&function.signature)))
        .collect()
}

/// Since overloading is only possible for different signatures, this should yield distinct names.
pub fn mangle_signature(signature: &str) -> String {
    signature
        .trim()
        .replace("(", "_")
        .replace(")", "")
        .replace(",", "_")
        .into()
}

#[cfg(test)]
mod tests {
    use crate::abi::mangle_signature;

    #[test]
    fn signature_mangling() {
        assert_eq!(mangle_signature("foo(int256,bool)"), "foo_int256_bool");
    }
}
