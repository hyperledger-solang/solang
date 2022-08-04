// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::sema::yul::tests::parse;

#[test]
fn function_inside_init() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                let x := 0
                for {
                    let i := 0
                    function tryThis(a, v) -> ret : u256 {
                        ret := mulmod(a, 2, v)
                    }
                } lt(i, 0x100) {
                    i := add(i, 0x20)
                } {
                    x := add(x, mload(i))
                    if lt(x, 30) {
                        continue
                    }
                    x := add(x, 1)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function definitions are not allowed inside for-init block"));
}

#[test]
fn unreachable_execution() {
    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            invalid()
            for {let i := 11
            } lt(i, 10) {i := add(i, 1)
        } {
                a := shr(i, 2)
                let b := shr(6, 5)
                    a := mul(a, b)
                //stop()
            }
            let x := 5
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message("unreachable yul statement"));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            for {let i := 11
                invalid()
            } lt(i, 10) {i := add(i, 1)
        } {
                a := shr(i, 2)
                let b := shr(6, 5)
                    a := mul(a, b)
                //stop()
            }
            let x := 5
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message("unreachable yul statement"));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            for {let i := 11
            } lt(i, 10) {i := add(i, 1)
            //stop()
        } {
                a := shr(i, 2)
                let b := shr(6, 5)
                    a := mul(a, b)
                invalid()
            }
            let x := 5
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message("unreachable yul statement"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {

            let a := 0
            for {let i := 11
            } lt(i, 10) {i := add(i, 1)
            invalid()
        } {
                a := shr(i, 2)
                let b := shr(6, 5)
                    a := mul(a, b)
                //stop()
            }
            let x := 5
        }
    }
}
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'x' has never been read"));
}
