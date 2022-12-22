// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity, AccountState, BorshToken};

#[test]
fn use_authority() {
    let mut vm = build_solidity(include_str!("../../docs/examples/solana/use_authority.sol"));

    let authority = account_new();

    vm.account_data.insert(
        authority,
        AccountState {
            data: vec![],
            owner: Some([0u8; 32]),
            lamports: 0,
        },
    );

    vm.constructor(&[BorshToken::Address(authority)]);

    let res = vm.function_must_fail("inc", &[]).unwrap();
    assert_ne!(res, 0);

    let res = vm.function("get", &[]).unwrap();
    assert_eq!(
        res,
        BorshToken::Uint {
            width: 64,
            value: 0.into()
        }
    );

    let mut metas = vm.default_metas();

    // "sign" the transaction with the authority
    if let Some(meta) = metas.iter_mut().find(|e| e.pubkey.0 == authority) {
        meta.is_signer = true;
    }

    vm.function_metas(&metas, "inc", &[]);

    let res = vm.function("get", &[]).unwrap();
    assert_eq!(
        res,
        BorshToken::Uint {
            width: 64,
            value: 1.into()
        }
    );
}
