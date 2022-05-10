use ethabi::{ethereum_types::U256, Token, Uint};
use crate::build_solidity;

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
      "#
    );

    vm.constructor("testing", &[], 0);

    let returns = vm.function("test_slot", &[], &[], 0, None);
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(56016))]
    );

    let returns = vm.function("call_data_array",
    &[
        Token::Array(vec![Token::Uint(U256::from(3)),
                                      Token::Uint(U256::from(5)),
                                      Token::Uint(U256::from(7)),
                    Token::Uint(U256::from(11)),
        ]),
    ],
        &[],
        0,
        None
    );
    assert_eq!(
        returns,
        vec![Token::Uint(U256::from(12884901888u64)), Token::Uint(U256::from(4))]
    );

    let returns = vm.function("selector_address", &[], &[], 0, None);
    // How to check if the address is correct?
    assert_eq!(
        returns[1],
        Token::Uint(U256::from(2081714652u32))
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

            h := a
            // if gt(a, 10) {
            //     g, h := mix(g, 10)
            // }
        }

        return (g, h);
    }
}
      "#
    );

    vm.constructor("testing", &[], 0);

    // let returns = vm.function("general_test", &[Token::Uint(Uint::from(5))], &[], 0, None);
    // assert_eq!(
    //     returns,
    //     vec![Token::Uint(Uint::from(100)), Token::Uint(Uint::from(20))]
    // );

    let returns = vm.function("general_test", &[Token::Uint(Uint::from(78))], &[], 0, None);

    std::println!("{:?}", returns);
}