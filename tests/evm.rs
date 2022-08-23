// SPDX-License-Identifier: Apache-2.0

use solang::{file_resolver::FileResolver, parse_and_resolve, sema::ast, Target};
use std::ffi::OsStr;

fn test_solidity(src: &str) -> ast::Namespace {
    let mut cache = FileResolver::new();

    cache.set_file_contents("test.sol", src.to_string());

    let ns = parse_and_resolve(OsStr::new("test.sol"), &mut cache, Target::EVM);

    ns.print_diagnostics_in_plain(&cache, false);

    ns
}

#[test]
fn address() {
    let ns = test_solidity(
        "
        contract address_tester {
            function encode_const() public returns (address) {
                return 0x52908400098527886E0F7030069857D2E4169EE7;
            }

            function test_arg(address foo) public {
                assert(foo == 0x27b1fdb04752bbc536007a920d24acb045561c26);

                // this literal is a number
                int x = 0x27b1fdb047_52bbc536007a920d24acb045561C26;
                assert(int(foo) == x);
            }

            function allones() public returns (address) {
                return address(1);
            }
        }",
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn try_catch() {
    let ns = test_solidity(
        r##"
        contract b {
            int32 x;

            constructor(int32 a) public {
                x = a;
            }

            function get_x(int32 t) public returns (int32) {
                if (t == 0) {
                    revert("cannot be zero");
                }
                return x * t;
            }
        }

        contract c {
            b x;

            constructor() public {
                x = new b(102);
            }

            function test() public returns (int32) {
                int32 state = 0;
                try x.get_x(0) returns (int32 l) {
                    state = 1;
                } catch Error(string err) {
                    if (err == "cannot be zero") {
                        state = 2;
                    } else {
                        state = 3;
                    }
                } catch (bytes ) {
                    state = 4;
                }

                return state;
            }
        }"##,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn selfdestruct() {
    let ns = test_solidity(
        r##"
        contract other {
            function goaway(address payable recipient) public returns (bool) {
                selfdestruct(recipient);
            }
        }

        contract c {
            other o;
            function step1() public {
                o = new other{value: 511}();
            }

            function step2() public {
                bool foo = o.goaway(payable(address(this)));
            }
        }"##,
    );

    assert!(!ns.diagnostics.any_errors());
}

#[test]
fn eth_builtins() {
    let ns = test_solidity(
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

    assert!(!ns.diagnostics.any_errors());
}
