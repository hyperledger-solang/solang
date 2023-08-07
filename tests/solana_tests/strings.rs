// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;
use num_traits::{One, Zero};

#[test]
fn storage_string_length() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        string st;
        function setString(string input) public {
            st = input;
        }

        function getLength() public view returns (uint32) {
            return st.length;
        }
    }
    "#,
    );
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let _ = vm
        .function("setString")
        .arguments(&[BorshToken::String("coffee_tastes_good".to_string())])
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm
        .function("getLength")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();

    assert_eq!(
        returns,
        BorshToken::Uint {
            width: 32,
            value: BigInt::from(18u8),
        }
    );
}

#[test]
fn load_string_vector() {
    let mut vm = build_solidity(
        r#"
    contract Testing {
        string[] string_vec;
        function testLength() public returns (uint32, uint32, uint32) {
            string_vec.push("tea");
            string_vec.push("coffe");
            string_vec.push("sixsix");
            string[] memory rr = string_vec;
            return (rr[0].length, rr[1].length, rr[2].length);
        }

        function getString(uint32 index) public view returns (string memory) {
            string[] memory rr = string_vec;
            return rr[index];
        }
    }
      "#,
    );

    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();
    let returns = vm
        .function("testLength")
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap()
        .unwrap_tuple();

    assert_eq!(
        returns[0],
        BorshToken::Uint {
            width: 32,
            value: BigInt::from(3u8),
        }
    );
    assert_eq!(
        returns[1],
        BorshToken::Uint {
            width: 32,
            value: BigInt::from(5u8),
        }
    );
    assert_eq!(
        returns[2],
        BorshToken::Uint {
            width: 32,
            value: BigInt::from(6u8),
        }
    );

    let returns = vm
        .function("getString")
        .arguments(&[BorshToken::Uint {
            width: 32,
            value: BigInt::zero(),
        }])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(returns, BorshToken::String("tea".to_string()));

    let returns = vm
        .function("getString")
        .arguments(&[BorshToken::Uint {
            width: 32,
            value: BigInt::one(),
        }])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(returns, BorshToken::String("coffe".to_string()));

    let returns = vm
        .function("getString")
        .arguments(&[BorshToken::Uint {
            width: 32,
            value: BigInt::from(2u8),
        }])
        .accounts(vec![("dataAccount", data_account)])
        .call()
        .unwrap();
    assert_eq!(returns, BorshToken::String("sixsix".to_string()));
}
