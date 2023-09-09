// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use std::str::FromStr;

#[test]
fn safe_math() {
    let mut vm = build_solidity(
        r#"
        library SafeMath {
            function add(uint x, uint y) internal pure returns (uint z) {
                require((z = x + y) >= x, 'ds-math-add-overflow');
            }

            function sub(uint x, uint y) internal pure returns (uint z) {
                require((z = x - y) <= x, 'ds-math-sub-underflow');
            }

            function mul(uint x, uint y) internal pure returns (uint z) {
                require(y == 0 || (z = x * y) / y == x, 'ds-math-mul-overflow');
            }
        }

        contract math {
            using SafeMath for uint;

            function mul_test(uint a, uint b) public returns (uint) {
                return a.mul(b);
            }

            function add_test(uint a, uint b) public returns (uint) {
                return a.add(b);
            }

            function sub_test(uint a, uint b) public returns (uint) {
                return a.sub(b);
            }
        }"#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("mul_test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("1000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("4000000000000000000").unwrap(),
            },
        ])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from_str("4000000000000000000000000000000000000").unwrap(),
        },
    );

    let returns = vm
        .function("add_test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("1000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("4000000000000000000").unwrap(),
            },
        ])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from_str("5000000000000000000").unwrap(),
        },
    );

    let returns = vm
        .function("sub_test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("4000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("1000000000000000000").unwrap(),
            },
        ])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from_str("3000000000000000000").unwrap(),
        },
    );

    vm.function("mul_test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("400000000000000000000000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("400000000000000000000000000000000000000").unwrap(),
            },
        ])
        .must_fail();

    vm.function(
        "add_test")
        .arguments(
        &[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("100000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("100000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            },
        ],
    )
        .must_fail();

    vm.function("sub_test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("1000000000000000000").unwrap(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_str("4000000000000000000").unwrap(),
            },
        ])
        .must_fail();
}
