// SPDX-License-Identifier: Apache-2.0

use crate::{account_new, build_solidity};
use byte_slice_cast::AsByteSlice;
use ethabi::{ethereum_types::U256, FixedBytes, Token, Uint};

#[test]
fn suffixes_access() {
    let mut vm = build_solidity(
        r#"

contract testing  {
    struct stru_test {
        string a;
        uint b;
    }

    stru_test ss1; // slot: 16
    stru_test ss2; // slot: 56
    function test_slot() public view returns (uint256) {
        uint256 ret = 98;
        stru_test storage local_t = ss2;
        assembly {
            let a := ss1.slot
            let b := mul(local_t.slot, 1000)
            ret := add(a, b)
            // offset should always be zero
            ret := sub(ret, ss2.offset)
            ret := sub(ret, local_t.offset)
        }

        return ret;
    }

    function call_data_array(uint32[] calldata vl) public pure returns (uint256, uint256) {
        uint256 ret1 = 98;
        uint256 ret2 = 99;
        assembly {
            let a := vl.offset
            let b := vl.length
            ret1 := a
            ret2 := b
        }

        return (ret1, ret2);
    }

    function selector_address() public view returns(uint256, uint256) {
        function () external returns (uint256) fPtr = this.test_slot;
        uint256 ret1 = 256;
        uint256 ret2 = 129;
        assembly {
            ret1 := fPtr.address
            ret2 := fPtr.selector
        }

        return (ret1, ret2);
    }
}
      "#,
    );

    vm.constructor("testing", &[]);

    let returns = vm.function("test_slot", &[], &[], None);
    assert_eq!(returns, vec![Token::Uint(U256::from(56016))]);

    let returns = vm.function(
        "call_data_array",
        &[Token::Array(vec![
            Token::Uint(U256::from(3)),
            Token::Uint(U256::from(5)),
            Token::Uint(U256::from(7)),
            Token::Uint(U256::from(11)),
        ])],
        &[],
        None,
    );

    assert_eq!(
        returns,
        vec![
            // the heap is 0x300000000. The header 32 bytes (sizeof(chunk) in heap.c)
            Token::Uint(U256::from(0x300000020u64)),
            Token::Uint(U256::from(4))
        ]
    );

    let returns = vm.function("selector_address", &[], &[], None);
    assert_eq!(
        returns,
        vec![
            Token::Uint(U256::from_big_endian(vm.stack[0].data.as_ref())),
            Token::Uint(U256::from(2081714652u32))
        ]
    );
}

#[test]
fn general_test() {
    let mut vm = build_solidity(
        r#"
contract testing  {
    function general_test(uint64 a) public view returns (uint64, uint256) {
        uint64 g = 0;
        uint256 h = 0;
        assembly {
            function sum(a, b) -> ret1 {
                ret1 := add(a, b)
            }

            function mix(a, b) -> ret1, ret2 {
                ret1 := mul(a, b)
                ret2 := add(a, b)
            }

            for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                if eq(a, 259) {
                    break
                }
                g := sum(g, 2)
                if gt(a, 10) {
                    continue
                }
                g := sub(g, 1)
            }

            if or(lt(a, 10), eq(a, 259)) {
                g, h := mix(g, 10)
            }
        }

        return (g, h);
    }
}
      "#,
    );

    vm.constructor("testing", &[]);

    let returns = vm.function("general_test", &[Token::Uint(Uint::from(5))], &[], None);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from(100)), Token::Uint(Uint::from(20))]
    );

    let returns = vm.function("general_test", &[Token::Uint(Uint::from(78))], &[], None);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from(20)), Token::Uint(Uint::from(0))]
    );

    let returns = vm.function("general_test", &[Token::Uint(Uint::from(259))], &[], None);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from(0)), Token::Uint(Uint::from(10))]
    );
}

#[test]
fn byte_builtin() {
    let mut vm = build_solidity(
        r#"
contract c {
	function getByte(uint256 bb) public pure returns (uint256) {
		uint256 ret = 0;
		assembly {
			ret := byte(5, bb)
		}
		return ret;
	}

	function divide(uint256 a, uint256 b) public pure returns (uint256 ret1, uint256 ret2) {
		assembly {
			ret1 := div(a, b)
			ret2 := mod(a, b)
		}
	}

	function mods(uint256 a, uint256 b, uint256 c) public pure returns (uint256 ret1, uint256 ret2) {
		assembly {
			ret1 := addmod(a, b, c)
			ret2 := mulmod(a, b, c)
		}
	}
}
        "#,
    );

    vm.constructor("c", &[]);
    let num: Vec<u8> = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x11, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
        0x2f, 0x31,
    ];
    let returns = vm.function(
        "getByte",
        &[Token::Uint(U256::from_big_endian(&num))],
        &[],
        None,
    );
    assert_eq!(returns, vec![Token::Uint(U256::from(6))]);

    let returns = vm.function(
        "divide",
        &[Token::Uint(U256::from(4)), Token::Uint(U256::from(3))],
        &[],
        None,
    );
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(1)), Token::Uint(U256::from(1))]
    );

    let returns = vm.function(
        "divide",
        &[Token::Uint(U256::from(4)), Token::Uint(U256::from(0))],
        &[],
        None,
    );
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(0)), Token::Uint(U256::from(0))]
    );

    let returns = vm.function(
        "mods",
        &[
            Token::Uint(U256::from(4)),
            Token::Uint(U256::from(2)),
            Token::Uint(U256::from(3)),
        ],
        &[],
        None,
    );
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(0)), Token::Uint(U256::from(2))]
    );

    let returns = vm.function(
        "mods",
        &[
            Token::Uint(U256::from(4)),
            Token::Uint(U256::from(2)),
            Token::Uint(U256::from(0)),
        ],
        &[],
        None,
    );
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(0)), Token::Uint(U256::from(0))]
    );
}

#[test]
fn external_function() {
    let mut vm = build_solidity(
        r#"
    contract C {

        function myFun() public {

        }

        function test(uint256 newAddress, bytes4 newSelector) public view returns (bytes4, address) {
            function() external fun = this.myFun;
            address myAddr = address(newAddress);
            assembly {
                fun.selector := newSelector
                fun.address  := myAddr
            }

            return (fun.selector, fun.address);
        }
    }
        "#,
    );

    vm.constructor("C", &[]);
    let mut addr: Vec<u8> = vec![0; 32];
    addr[5] = 90;
    let returns = vm.function(
        "test",
        &[
            Token::Uint(U256::from_little_endian(&addr[..])),
            Token::FixedBytes(FixedBytes::from([1, 2, 3, 4])),
        ],
        &[],
        None,
    );

    let selector = returns[0].clone().into_fixed_bytes().unwrap();
    assert_eq!(selector, vec![1, 2, 3, 4]);
    let addr = returns[1].clone().into_fixed_bytes().unwrap();
    assert_eq!(addr[26], 90);
}

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
}"#,
    );

    runtime.constructor("testing", &[]);
    let returns = runtime.function("test_address", &[], &[], None);
    let addr = returns[0].clone().into_uint().unwrap();
    let mut b_vec = addr.as_byte_slice().to_vec();
    b_vec.reverse();
    assert_eq!(&b_vec, runtime.stack[0].data.as_ref());

    runtime
        .account_data
        .get_mut(&runtime.stack[0].data)
        .unwrap()
        .lamports = 102;
    let returns = runtime.function("test_balance", &[], &[], None);
    assert_eq!(returns, vec![Token::Uint(U256::from(102))]);

    let returns = runtime.function("test_selfbalance", &[], &[], None);
    assert_eq!(returns, vec![Token::Uint(U256::from(102))]);

    let sender = account_new();

    let returns = runtime.function("test_caller", &[], &[], Some(&sender));
    let addr = returns[0].clone().into_uint().unwrap().0;
    let mut b_vec = addr.as_byte_slice().to_vec();
    b_vec.reverse();
    assert_eq!(b_vec, sender.to_vec());
}

#[test]
fn addmod_mulmod() {
    let mut vm = build_solidity(
        r#"
    contract foo {
        function testMod() public pure returns (uint256 a, uint256 b) {
            assembly {
                let x := 115792089237316195423570985008687907853269984665640564039457584007913129639935
                let y := 115792089237316195423570985008687907853269984665640564039457584007913129639935

                a := mulmod(x, 2, 10)
                b := addmod(y, 2, 10)
            }

            return (a, b);
        }
    }
        "#,
    );

    vm.constructor("foo", &[]);

    let returns = vm.function("testMod", &[], &[], None);
    assert_eq!(
        returns,
        vec![Token::Uint(Uint::from(0)), Token::Uint(Uint::from(7)),]
    );
}
