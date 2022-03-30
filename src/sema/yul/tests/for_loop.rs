#![cfg(test)]

use crate::sema::yul::tests::{assert_message_in_diagnostics, parse};

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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "function definitions are not allowed inside for-init block"
    ));
}

#[test]
fn unreachable_execution() {
    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            stop()
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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable yul statement"
    ));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            for {let i := 11
                stop()
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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable yul statement"
    ));

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
                stop()
            }
            let x := 5
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable yul statement"
    ));

    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {

            let a := 0
            for {let i := 11
            } lt(i, 10) {i := add(i, 1)
            stop()
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
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "yul variable ‘x‘ has never been read"
    ));
}
