// SPDX-License-Identifier: Apache-2.0

use convert_case::{Boundary, Case, Casing};
use sha2::{Digest, Sha256};

/// Generate discriminator based on the name of the function. This is the 8 byte
/// value anchor uses to dispatch function calls on. This should match
/// anchor's behaviour - we need to match the discriminator exactly
pub fn discriminator(namespace: &'static str, name: &str) -> Vec<u8> {
    let mut hasher = Sha256::new();
    // must match snake-case npm library, see
    // https://github.com/coral-xyz/anchor/blob/master/ts/packages/anchor/src/coder/borsh/instruction.ts#L389
    let normalized = name
        .from_case(Case::Camel)
        .without_boundaries(&[Boundary::LowerDigit])
        .to_case(Case::Snake);
    hasher.update(format!("{}:{}", namespace, normalized));
    hasher.finalize()[..8].to_vec()
}
