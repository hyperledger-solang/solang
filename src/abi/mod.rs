
use resolver::{Contract, Target};

pub mod ethabi;
mod substrate;

pub fn generate_abi(contract: &Contract, verbose: bool) -> (Vec<u8>, &'static str) {
    match contract.target {
        Target::Burrow => {
            if verbose {
                eprintln!("info: Generating Burrow ABI for contract {}", contract.name);
            }

            let abi = ethabi::gen_abi(contract);

            (serde_json::to_string(&abi).unwrap().as_bytes().to_vec(), "abi")
        },
        Target::Substrate => {
            if verbose {
                eprintln!("info: Generating Substrate ABI for contract {}", contract.name);
            }

            let abi = substrate::gen_abi(contract);

            (serde_json::to_string_pretty(&abi).unwrap().as_bytes().to_vec(), "json")
        }
    }
}