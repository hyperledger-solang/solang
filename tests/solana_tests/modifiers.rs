// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;

#[test]
fn returns_and_phis_needed() {
    let mut vm = build_solidity(
        r#"
        contract c {
            int foo;
            bool bar;

            function func(bool cond) external mod(cond) returns (int, bool) {
                return (foo, bar);
            }

            modifier mod(bool cond) {
                bar = cond;
                if (cond) {
                    foo = 12;
                    _;
                } else {
                    foo = 40;
                    _;
                }
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("func")
        .arguments(&[BorshToken::Bool(false)])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::from(40u8),
            },
            BorshToken::Bool(false)
        ]
    );

    let returns = vm
        .function("func")
        .arguments(&[BorshToken::Bool(true)])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Int {
                width: 256,
                value: BigInt::from(12u8)
            },
            BorshToken::Bool(true)
        ]
    );
}
