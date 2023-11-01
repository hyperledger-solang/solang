// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::{BigInt, Sign};
use num_traits::{One, Zero};

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

    function call_data_array(uint32[] calldata vl) public view returns (uint256, uint256) {
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("test_slot")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(56016u16)
        }
    );

    let returns = vm
        .function("call_data_array")
        .arguments(&[BorshToken::Array(vec![
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(3u8),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(5u8),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(7u8),
            },
            BorshToken::Uint {
                width: 32,
                value: BigInt::from(11u8),
            },
        ])])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            // the heap is 0x300000000. The header 24 bytes (sizeof(chunk) in heap.c)
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(0x300000018u64)
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8)
            },
        ]
    );

    let returns = vm
        .function("selector_address")
        .call()
        .unwrap()
        .unwrap_tuple();

    let program_id = vm.stack[0].id;
    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_bytes_be(Sign::Plus, program_id.as_ref())
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(799097422081508461u64)
            },
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("general_test")
        .arguments(&[BorshToken::Uint {
            width: 64,
            value: BigInt::from(5u8),
        }])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(100u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(20u8)
            },
        ]
    );

    let returns = vm
        .function("general_test")
        .arguments(&[BorshToken::Uint {
            width: 64,
            value: BigInt::from(78u8),
        }])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::from(20u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
        ]
    );

    let returns = vm
        .function("general_test")
        .arguments(&[BorshToken::Uint {
            width: 64,
            value: BigInt::from(259u16),
        }])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 64,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(10u8)
            },
        ]
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let num: Vec<u8> = vec![
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f,
        0x11, 0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28, 0x29, 0x2a, 0x2b, 0x2c, 0x2d, 0x2e,
        0x2f, 0x31,
    ];

    let returns = vm
        .function("getByte")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from_bytes_be(Sign::Plus, &num),
        }])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(6u8),
        }
    );

    let returns = vm
        .function("divide")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(3u8),
            },
        ])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::one(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::one(),
            },
        ]
    );

    let returns = vm
        .function("divide")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
        ])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
        ]
    );

    let returns = vm
        .function("mods")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(3u8),
            },
        ])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
        ]
    );

    let returns = vm
        .function("mods")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(4u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(2u8),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
        ])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
        ]
    );
}

#[test]
fn external_function() {
    let mut vm = build_solidity(
        r#"
    contract C {

        function myFun() public {

        }

        function test(uint256 newAddress, bytes8 newSelector) public view returns (bytes8, address) {
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let mut addr: Vec<u8> = vec![0; 32];
    addr[5] = 90;
    let returns = vm
        .function("test")
        .arguments(&[
            BorshToken::Uint {
                width: 256,
                value: BigInt::from_bytes_le(Sign::Plus, addr.as_slice()),
            },
            BorshToken::FixedBytes(vec![1, 2, 3, 4, 5, 6, 7, 8]),
        ])
        .call()
        .unwrap()
        .unwrap_tuple();

    let selector = returns[0].clone().into_fixed_bytes().unwrap();
    assert_eq!(selector, vec![1, 2, 3, 4, 5, 6, 7, 8]);
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
}"#,
    );

    let data_account = runtime.initialize_data_account();
    let program_id = runtime.stack[0].id;
    runtime
        .function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = runtime.function("test_address").call().unwrap();
    let addr = returns.into_bigint().unwrap();
    let mut b_vec = addr.to_bytes_be().1;
    // test_address() returns address as uint256. If the highest bits are 0, then addr.to_bytes_be().1
    // may not return 32 bytes.
    while b_vec.len() < 32 {
        b_vec.insert(0, 0);
    }
    assert_eq!(&b_vec, program_id.as_ref());
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

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm.function("testMod").call().unwrap().unwrap_tuple();

    assert_eq!(
        returns,
        vec![
            BorshToken::Uint {
                width: 256,
                value: BigInt::zero(),
            },
            BorshToken::Uint {
                width: 256,
                value: BigInt::from(7u8),
            },
        ]
    );
}

#[test]
fn switch_statement() {
    let mut vm = build_solidity(
        r#"

contract Testing {
    function switch_default(uint a) public pure returns (uint b) {
        b = 4;
        assembly {
            switch a
            case 1 {
                b := 5
            }
            case 2 {
                b := 6
            }
            default {
                b := 7
            }
        }

        if (b == 7) {
            b += 2;
        }
    }

    function switch_no_default(uint a) public pure returns (uint b) {
        b = 4;
        assembly {
            switch a
            case 1 {
                b := 5
            }
            case 2 {
                b := 6
            }
        }

        if (b == 5) {
            b -= 2;
        }
    }

    function switch_no_case(uint a) public pure returns (uint b) {
        b = 7;
        assembly {
            switch a
            default {
                b := 5
            }
        }

        if (b == 5) {
            b -= 1;
        }
    }
}
        "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let returns = vm
        .function("switch_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::one(),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(5u8),
        }
    );

    let returns = vm
        .function("switch_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(6u8),
        },
    );

    let returns = vm
        .function("switch_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(6u8),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(9u8),
        }
    );

    let returns = vm
        .function("switch_no_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::one(),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8),
        },
    );

    let returns = vm
        .function("switch_no_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(2u8),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(6u8),
        },
    );

    let returns = vm
        .function("switch_no_default")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(6u8),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(4u8),
        },
    );

    let returns = vm
        .function("switch_no_case")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(3u8),
        }])
        .call()
        .unwrap();
    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 256,
            value: BigInt::from(4u8),
        },
    );
}
