// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;

/// This tests check that a public storage variable is not eliminated
/// and that an assignment inside an expression works
#[test]
fn test_returns() {
    let file = r#"
    contract c1 {
        int public pb1;

        function assign() public {
            pb1 = 5;
        }

        int t1;
        int t2;
        function test1() public returns (int) {
            t1 = 2;
            t2 = 3;
            int f = 6;
            int c = 32 +4 *(f = t1+t2);
            return c;
        }

        function test2() public returns (int) {
            t1 = 2;
            t2 = 3;
            int f = 6;
            int c = 32 + 4*(f= t1+t2);
            return f;
        }

    }
    "#;

    let mut vm = build_solidity(file);
    vm.constructor_with_borsh("c1", &[]);
    let _ = vm.function_with_borsh("assign", &[], &[], None);
    let returns = vm.function_with_borsh("pb1", &[], &[], None);

    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 256,
            value: BigInt::from(5u8)
        }]
    );

    let returns = vm.function_with_borsh("test1", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 256,
            value: BigInt::from(52u8)
        }]
    );
    let returns = vm.function_with_borsh("test2", &[], &[], None);
    assert_eq!(
        returns,
        vec![BorshToken::Int {
            width: 256,
            value: BigInt::from(5u8)
        }]
    );
}
