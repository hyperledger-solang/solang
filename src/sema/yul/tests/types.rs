// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::sema::yul::tests::parse;

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
    assert!(ns
        .diagnostics
        .contains_message("the specified type 's120' does not exist"));
}

#[test]
fn incompatible_argument() {
    let file = r#"
contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(invalid(), 5)
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("builtin function 'invalid' returns nothing"));

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
    assert!(ns
        .diagnostics
        .contains_message("function 'doThis' returns nothing"));

    assert!(ns
        .diagnostics
        .contains_message("function 'foo' has multiple returns and cannot be used in this scope"));
}
