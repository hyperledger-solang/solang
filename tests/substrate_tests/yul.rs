// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};
use primitive_types::U256;

#[derive(Debug, Encode, Decode)]
struct Val256(U256);

#[test]
fn assign_suffixes() {
    #[derive(Debug, Encode, Decode)]
    struct CallDataInput {
        pub input: U256,
        pub vec: Vec<u32>,
    }

    let mut runtime = build_solidity(
        r#"
contract testing  {
    struct stru_test {
        string a;
        uint b;
    }


    stru_test t_s_t;
    function test_local_vec(uint256 input) public pure returns (uint256 ret) {
        uint256[] vec;
        vec.push(uint256(4));
        assembly {
            vec := add(input, 7)
            input := 6
            ret := vec
        }
    }

    function test_struct(uint256 input) public pure returns (uint256 ret) {
        stru_test tt = stru_test({a: "tea", b: 5});
        assembly {
            tt := add(input, 7)
            input := 6
            ret := tt
        }
    }

    function test_mem_vec(uint256 input) public pure returns (uint256 ret) {
        uint256[] memory vec;
        vec.push(uint256(4));
        assembly {
            vec := add(input, 7)
            input := 9
            ret := vec
        }
    }

    function test_mem_struct(uint256 input) public pure returns (uint256 ret) {
        stru_test memory tt = stru_test({a: "tea", b: 5});
        assembly {
            tt := add(input, 7)
            input := 9
            ret := tt
        }
    }

    function calldata_vec(uint256 input, uint32[] calldata vec) public pure returns (uint256 ret) {
        assembly {
            vec.offset := add(input, 7)
            input := 9
            ret := vec.offset
        }
    }

    function storage_struct(uint256 input) public returns (uint256 ret) {
        stru_test storage local_t = t_s_t;
        assembly {
            local_t.slot := add(input, 7)
            input := 90
            ret := local_t.slot
        }
    }
}
      "#,
    );

    runtime.function("test_local_vec", Val256(U256::from(7)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(14)).encode());

    runtime.function("test_struct", Val256(U256::from(20)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(27)).encode());

    runtime.function("test_mem_vec", Val256(U256::from(30)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(37)).encode());

    runtime.function("test_mem_struct", Val256(U256::from(8)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(15)).encode());

    runtime.function(
        "calldata_vec",
        CallDataInput {
            input: U256::from(19),
            vec: vec![0, 1, 2, 3, 4],
        }
        .encode(),
    );
    assert_eq!(runtime.output(), Val256(U256::from(26)).encode());

    runtime.function("storage_struct", Val256(U256::from(17)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(24)).encode());
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

    function test_callvalue() public payable returns (uint256 ret) {
        assembly {
            let a := callvalue()
            ret := a
        }
    }
}"#,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test_address", Vec::new());
    let mut b_vec = runtime.output().to_vec();
    b_vec.reverse();
    assert_eq!(b_vec, runtime.account.to_vec());

    runtime.function("test_balance", Vec::new());
    assert_eq!(
        runtime.output()[..16].to_vec(),
        runtime.contracts[0].value.encode()
    );

    runtime.function("test_selfbalance", Vec::new());
    assert_eq!(
        runtime.output()[..16].to_vec(),
        runtime.contracts[0].value.encode()
    );

    runtime.function("test_caller", Vec::new());
    let mut b_vec = runtime.output().to_vec();
    b_vec.reverse();
    assert_eq!(b_vec, runtime.caller().to_vec());

    let selector = runtime.contracts[0].messages["test_callvalue"].clone();
    runtime.raw_function(selector, 0xdeadcafe);
    let mut expected = 0xdeadcafeu32.to_le_bytes().to_vec();
    expected.resize(32, 0);
    assert_eq!(runtime.output(), expected);
}

#[test]
fn switch_statement() {
    let mut runtime = build_solidity(
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

    runtime.constructor(0, Vec::new());

    runtime.function("switch_default", Val256(U256::from(1)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(5)).encode());

    runtime.function("switch_default", Val256(U256::from(2)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(6)).encode());

    runtime.function("switch_default", Val256(U256::from(6)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(9)).encode());

    runtime.function("switch_no_default", Val256(U256::from(1)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(3)).encode());

    runtime.function("switch_no_default", Val256(U256::from(2)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(6)).encode());

    runtime.function("switch_no_default", Val256(U256::from(6)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(4)).encode());

    runtime.function("switch_no_case", Val256(U256::from(3)).encode());
    assert_eq!(runtime.output(), Val256(U256::from(4)).encode());
}
