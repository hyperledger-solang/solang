use sema::ast::Namespace;
use Target;

pub mod ethereum;
pub mod substrate;

pub fn generate_abi(contract_no: usize, ns: &Namespace, verbose: bool) -> (String, &'static str) {
    match ns.target {
        Target::Substrate => {
            if verbose {
                eprintln!(
                    "info: Generating Substrate ABI for contract {}",
                    ns.contracts[contract_no].name
                );
            }

            let abi = substrate::gen_abi(contract_no, ns);

            (serde_json::to_string_pretty(&abi).unwrap(), "json")
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
