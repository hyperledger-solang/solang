use crate::build_solidity;
use ethabi::{ethereum_types::U256, Token};
use num_bigint::BigUint;
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

    vm.constructor("math", &[]);

    let returns = vm.function(
        "mul_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("1000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("4000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Uint(biguint_to_eth(
            &BigUint::from_str("4000000000000000000000000000000000000").unwrap()
        ))]
    );

    let returns = vm.function(
        "add_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("1000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("4000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Uint(biguint_to_eth(
            &BigUint::from_str("5000000000000000000").unwrap()
        ))]
    );

    let returns = vm.function(
        "sub_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("4000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("1000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![Token::Uint(biguint_to_eth(
            &BigUint::from_str("3000000000000000000").unwrap()
        ))]
    );

    let res = vm.function_must_fail(
        "mul_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("400000000000000000000000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("400000000000000000000000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_ne!(res, Ok(0));

    let res = vm.function_must_fail(
        "add_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("100000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("100000000000000000000000000000000000000000000000000000000000000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_ne!(res, Ok(0));

    let res = vm.function_must_fail(
        "sub_test",
        &[
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("1000000000000000000").unwrap(),
            )),
            Token::Uint(biguint_to_eth(
                &BigUint::from_str("4000000000000000000").unwrap(),
            )),
        ],
        &[],
        None,
    );

    assert_ne!(res, Ok(0));
}

fn biguint_to_eth(v: &BigUint) -> U256 {
    let mut buf = v.to_bytes_be();
    let width = 32;

    while buf.len() > width {
        buf.remove(0);
    }

    while buf.len() < width {
        buf.insert(0, 0);
    }

    U256::from_big_endian(&buf)
}
