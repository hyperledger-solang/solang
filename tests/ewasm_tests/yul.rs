// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use byte_slice_cast::AsByteSlice;
use ethabi::{Token, Uint};

#[test]
fn eth_builtins() {
    let mut runtime = build_solidity(
        r#"
contract testing  {
    function test_address() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := a
        }
    }

    function test_balance() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := balance(a)
        }
    }

    function test_selfbalance() public view returns (uint256 ret) {
        assembly {
            let a := selfbalance()
            ret := a
        }
    }

    function test_caller() public view returns (uint256 ret) {
        assembly {
            let a := caller()
            ret := a
        }
    }

    function test_callvalue() public view returns (uint256 ret) {
        assembly {
            let a := callvalue()
            ret := a
        }
    }

    function test_extcodesize() public view returns (uint256 ret) {
        assembly {
            let a := address()
            ret := extcodesize(a)
        }
    }
}
"#,
    );

    runtime.constructor(&[]);
    let returns = runtime.function("test_address", &[]);
    let addr = returns[0].clone().into_uint().unwrap().0;
    let mut b_vec = addr.as_byte_slice().to_vec();
    b_vec.reverse();
    assert_eq!(&b_vec[12..32], runtime.vm.cur.as_ref());

    let returns = runtime.function("test_balance", &[]);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from_big_endian(
            runtime
                .accounts
                .get(runtime.vm.cur.as_ref())
                .unwrap()
                .1
                .to_be_bytes()
                .as_ref()
        ))]
    );

    let returns = runtime.function("test_selfbalance", &[]);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from_big_endian(
            runtime
                .accounts
                .get(runtime.vm.cur.as_ref())
                .unwrap()
                .1
                .to_be_bytes()
                .as_ref()
        ))]
    );

    let returns = runtime.function("test_caller", &[]);
    let addr = returns[0].clone().into_uint().unwrap().0;
    let mut b_vec = addr.as_byte_slice().to_vec();
    b_vec.reverse();
    assert_eq!(&b_vec[12..32], runtime.caller.as_slice());

    let returns = runtime.function("test_callvalue", &[]);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from_big_endian(
            runtime.value.to_be_bytes().as_ref()
        ))]
    );

    let returns = runtime.function("test_extcodesize", &[]);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from_big_endian(
            runtime.accounts[&runtime.vm.cur]
                .0
                .len()
                .to_be_bytes()
                .as_ref()
        ))]
    );
}
