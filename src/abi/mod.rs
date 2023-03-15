// SPDX-License-Identifier: Apache-2.0

use crate::sema::ast::Namespace;
use crate::Target;

pub mod anchor;
pub mod ethereum;
mod solana_accounts;
pub mod substrate;
mod tests;

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
                    "info: Generating Substrate metadata for contract {}",
                    ns.contracts[contract_no].name
                );
            }

            let metadata = substrate::metadata(contract_no, code, ns);

            (serde_json::to_string_pretty(&metadata).unwrap(), "contract")
        }
        Target::Solana => {
            if verbose {
                eprintln!(
                    "info: Generating Anchor metadata for contract {}",
                    ns.contracts[contract_no].name
                );
            }

            let idl = anchor::generate_anchor_idl(contract_no, ns);

            (serde_json::to_string_pretty(&idl).unwrap(), "json")
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
