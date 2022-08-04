// SPDX-License-Identifier: Apache-2.0

#![cfg(test)]

use crate::sema::yul::tests::parse;

#[test]
fn variables_assignment_mismatch() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let x, y, z := 2
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("a single value cannot be assigned to multiple variables"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let x, y, z := add(4, 0x40af)
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("3 variables on the left hand side, but the function returns 1 values"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let x, y, z := foo(4, 0x40af)

           function foo(a, b) -> ret1 : s32, ret2 : u64 {
               ret1 := add(a, b)
               ret2 := sub(b, a)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("3 variables on the left hand side, but the function returns 2 values"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let x, y := foo(4, 0x40af)

           y := foo(3, 2)

           function foo(a, b) -> ret1 : s32, ret2 : u64 {
               ret1 := add(a, b)
               ret2 := sub(b, a)
           }
        }
    }
}
    "#;

    let ns = parse(file);

    assert!(ns
        .diagnostics
        .contains_message("1 variables on the left hand side, but the function returns 2 values"));
}

#[test]
fn assignment() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let foo := 2

           function foo(a, b) -> ret1 : s32, ret2 : u64 {
               ret1 := add(a, b)
               ret2 := sub(b, a)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("name 'foo' has been defined as a function"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           let mulmod := 2

           function foo(a, b) -> ret1 : s32, ret2 : u64 {
               ret1 := add(a, b)
               ret2 := sub(b, a)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("'mulmod' is a built-in function and cannot be a variable name"));

    let file = r#"
    contract testTypes {
    function testAsm() public pure {
        assembly {
           let verbatim_2 := 2

        }
    }
}
    "#;

    let ns = parse(file);

    assert!(ns
        .diagnostics
        .contains_message("the prefix 'verbatim' is reserved for verbatim functions"));
}

#[test]
fn top_level_function_calls() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           mod(3, 4)
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("top level function calls must not return anything"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

           function tryThis(c) -> ret {
               ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("top level function calls must not return anything"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

           function tryThis(c) {
               let ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}

#[test]
fn leave_statement() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

            leave
           function tryThis(c) {
               let ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("leave statement cannot be used outside a function"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

           function tryThis(c) {
               let ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}

#[test]
fn continue_statement() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

            continue
           function tryThis(c) {
               let ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("continue statement outside a for loop"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                let x := 0
                for {
                    let i := 0
                } lt(i, 0x100) {
                    i := add(i, 0x20)
                } {
                    x := add(x, i)
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
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}

#[test]
fn break_statement() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
           tryThis(2)

            break
           function tryThis(c) {
               let ret := div(c, 3)
           }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("break statement outside a for loop"));

    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                let x := 0
                for {
                    let i := 0
                } lt(i, 0x100) {
                    i := add(i, 0x20)
                } {
                    x := add(x, i)
                    if lt(x, 30) {
                        break
                    }
                    x := add(x, 1)
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}
