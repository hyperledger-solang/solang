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

    vm.constructor("c", &[]);

    let returns = vm.function("func", &[BorshToken::Bool(false)], &[], None);

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

    let returns = vm.function("func", &[BorshToken::Bool(true)], &[], None);

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
