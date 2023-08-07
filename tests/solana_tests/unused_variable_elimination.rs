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
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let _ = vm
        .function("assign")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm
        .function("pb1")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Int {
            width: 256,
            value: BigInt::from(5u8)
        }
    );

    let returns = vm
        .function("test1")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Int {
            width: 256,
            value: BigInt::from(52u8)
        }
    );
    let returns = vm
        .function("test2")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Int {
            width: 256,
            value: BigInt::from(5u8)
        }
    );
}
