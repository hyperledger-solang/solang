#![cfg(test)]

use crate::sema::assembly::tests::{assert_message_in_diagnostics, parse};

#[test]
fn type_not_found() {
    let file = r#"
contract testTypes {
    function testAsm() public view {
        assembly {
            let x : s120 := 230
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "the specified type 's120' does not exist"
    ));
}

#[test]
fn incompatible_argument() {
    let file = r#"
contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(log0(0, 1), 5)
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "builtin function 'log0' returns nothing"
    ));

    let file = r#"
    contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(doThis(1), doThat(2))
            let y := add(x, foo(3))

            function doThis(a) {
                log0(a, 1)
            }

            function doThat(a) -> ret {
                ret := mul(a, 2)
            }

            function foo(a) -> ret1, ret2 {
                ret1 := mul(a, 2)
                ret2 := mul(a, 3)
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "function 'doThis' returns nothing"
    ));

    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "function 'foo' has multiple returns and cannot be used in this scope"
    ));
}
