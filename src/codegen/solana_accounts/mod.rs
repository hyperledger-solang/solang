// SPDX-License-Identifier: Apache-2.0

pub(super) mod account_collection;
pub(super) mod account_management;

use crate::sema::solana_accounts::BuiltinAccounts;
use base58::FromBase58;
use num_bigint::{BigInt, Sign};
use num_traits::Zero;
use once_cell::sync::Lazy;
use std::collections::HashMap;

/// If the public keys available in AVAILABLE_ACCOUNTS are hardcoded in a Solidity contract
/// for external calls, we can detect them and leverage Anchor's public key auto populate feature.
static AVAILABLE_ACCOUNTS: Lazy<HashMap<BigInt, BuiltinAccounts>> = Lazy::new(|| {
    HashMap::from([
        (BigInt::zero(), BuiltinAccounts::SystemAccount),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL"
                    .from_base58()
                    .unwrap(),
            ),
            BuiltinAccounts::AssociatedTokenProgram,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"SysvarRent111111111111111111111111111111111"
                    .from_base58()
                    .unwrap(),
            ),
            BuiltinAccounts::RentAccount,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
                    .from_base58()
                    .unwrap(),
            ),
            BuiltinAccounts::TokenProgramId,
        ),
        (
            BigInt::from_bytes_be(
                Sign::Plus,
                &"SysvarC1ock11111111111111111111111111111111"
                    .from_base58()
                    .unwrap(),
            ),
            BuiltinAccounts::ClockAccount,
        ),
    ])
});

/// Retrieve a name from an account, according to Anchor's constant accounts map
/// https://github.com/coral-xyz/anchor/blob/06c42327d4241e5f79c35bc5588ec0a6ad2fedeb/ts/packages/anchor/src/program/accounts-resolver.ts#L54-L60
fn account_from_number(num: &BigInt) -> Option<String> {
    AVAILABLE_ACCOUNTS.get(num).map(|e| e.to_string())
}
