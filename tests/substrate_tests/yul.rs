// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use ethabi::ethereum_types::U256;
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
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
    assert_eq!(runtime.vm.output, Val256(U256::from(14)).encode());

    runtime.function("test_struct", Val256(U256::from(20)).encode());
    assert_eq!(runtime.vm.output, Val256(U256::from(27)).encode());

    runtime.function("test_mem_vec", Val256(U256::from(30)).encode());
    assert_eq!(runtime.vm.output, Val256(U256::from(37)).encode());

    runtime.function("test_mem_struct", Val256(U256::from(8)).encode());
    assert_eq!(runtime.vm.output, Val256(U256::from(15)).encode());

    runtime.function(
        "calldata_vec",
        CallDataInput {
            input: U256::from(19),
            vec: vec![0, 1, 2, 3, 4],
        }
        .encode(),
    );
    assert_eq!(runtime.vm.output, Val256(U256::from(26)).encode());

    runtime.function("storage_struct", Val256(U256::from(17)).encode());
    assert_eq!(runtime.vm.output, Val256(U256::from(24)).encode());
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
    let mut b_vec = runtime.vm.output.to_vec();
    b_vec.reverse();
    assert_eq!(b_vec, runtime.vm.account.to_vec());

    runtime.function("test_balance", Vec::new());
    assert_eq!(
        runtime.vm.output[..16].to_vec(),
        runtime
            .accounts
            .get_mut(&runtime.vm.account)
            .unwrap()
            .1
            .encode(),
    );

    runtime.function("test_selfbalance", Vec::new());
    assert_eq!(
        runtime.vm.output[..16].to_vec(),
        runtime
            .accounts
            .get_mut(&runtime.vm.account)
            .unwrap()
            .1
            .encode(),
    );

    runtime.function("test_caller", Vec::new());
    let mut b_vec = runtime.vm.output.to_vec();
    b_vec.reverse();
    assert_eq!(b_vec, runtime.vm.caller.to_vec());

    runtime.vm.value = 0xdeadcafeu128;
    runtime.function("test_callvalue", Vec::new());

    let mut expected = 0xdeadcafeu32.to_le_bytes().to_vec();
    expected.resize(32, 0);

    assert_eq!(runtime.vm.output, expected);
}
