// SPDX-License-Identifier: Apache-2.0

use crate::sema::yul::tests::parse;

#[test]
fn repeated_names() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                function tryThis(a, a) {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    let b := add(a, 6)
                    return(b, 0)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("variable name 'a' already used in this scope"));

    let file = r#"
    contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                function tryThis(b, a) -> b {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    b := add(a, 6)
                    return(b, 0)
                }
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("variable name 'b' already used in this scope"));
}
