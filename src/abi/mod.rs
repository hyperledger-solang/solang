
use resolver::Contract;
use Target;

pub mod ethabi;
pub mod substrate;

pub fn generate_abi(contract: &Contract, verbose: bool) -> (String, &'static str) {
    match contract.target {
        Target::Ewasm |
        Target::Burrow => {
            if verbose {
                eprintln!("info: Generating Ethereum ABI for contract {}", contract.name);
            }

            let abi = ethabi::gen_abi(contract);

            (serde_json::to_string(&abi).unwrap(), "abi")
        },
        Target::Substrate => {
            if verbose {
                eprintln!("info: Generating Substrate ABI for contract {}", contract.name);
            }

            let abi = substrate::gen_abi(contract);

            (serde_json::to_string_pretty(&abi).unwrap(), "json")
        }
    }
}