mod bpf;
mod wasm;

use crate::Target;

/// Take an object file and turn it into a final linked binary ready for deployment
pub fn link(input: &[u8], name: &str, target: Target) -> Vec<u8> {
    if target == Target::Solana {
        bpf::link(input, name)
    } else {
        wasm::link(input, target)
    }
}
