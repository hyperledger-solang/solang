// SPDX-License-Identifier: Apache-2.0

use crate::sema::yul::tests::parse;

#[test]
fn inside_function() {
    let file = r#"
        contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly
            {
                function foo(a, b) -> ret {
                    let x := address()
                    ret := add(sub(b, x), a)
                }
            }
        }
    }
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn inside_argument() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                foo(balance(4), 5)
                function foo(a, b) {
                    let x := 5
                    let ret := add(sub(b, x), a)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let y := add(balance(4), 5)
                function foo(a, b) {
                    let x := 5
                    let ret := add(sub(b, x), a)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn block() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                {
                    foo(balance(4), 5)
                }
                function foo(a, b) {
                    let x := 5
                    let ret := add(sub(b, x), a)
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn assign_declaration() {
    let file = r#"
        contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                function foo(a, b) -> ret {
                    let x := 5
                    ret := balance(x)
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                function foo(a, b) -> ret {
                    let x := balance(4)
                    ret := x
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn if_block() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                if balance(5) {
                    invalid()
                }
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let x := 2
                if gt(x, 4) {
                    x := balance(4)
                    invalid()
                }
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn switch() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                switch balance(4)
                case 0 {let x := 5}
                default {
                    let y := 4
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let y := 9
                switch y
                case 0 {let x := balance(4)}
                default {
                    let p := 4
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                let y := 6
                switch y
                case 0 {let x := 5}
                default {
                    let p := balance(3)
                }
            }
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn test_for() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := balance(4)} gt(i, 0) {i := sub(i, 2)} {
                    let x := shr(i, 6)
                }
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := 6} balance(4) {i := sub(i, 2)} {
                    let x := shr(i, 6)
                }
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := 6} gt(i, 0) {i := balance(4)} {
                    let x := shr(i, 6)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := 6} gt(i, 0) {i := sub(i, 2)} {
                    let x := balance(4)
                }
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));
}

#[test]
fn pure_function() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := balance(4)} gt(i, 0) {i := sub(i, 2)} {
                    let x := shr(i, 6)
                }
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression reads from state"));

    let file = r#"
        contract testTypes {
    function testAsm(uint[] calldata vl) public pure {
        assembly {
            {
                for {let i := 6} gt(i, 0) {i := sub(i, 2)} {
                    log0(i, 0)
                }
            }
        }
    }
}    "#;

    let ns = parse(file);
    // TODO: There is no implemented function that writes to state
    assert!(ns
        .diagnostics
        .contains_message("function declared 'pure' but this expression writes to state"));
}

#[test]
fn view_function() {
    let file = r#"
    contract testTypes {
    function testAsm(uint[] calldata vl) public view {
        assembly {
            {
                for {let i := 6} gt(i, 0) {i := sub(i, 2)} {
                    let y := create(3, 2, 1)
                }
            }
        }
    }
}
    "#;

    // TODO: There is no implemented function that writes to the state, so there is no way to test mutability here
    let ns = parse(file);
    assert!(ns
        .diagnostics
        .contains_message("function declared 'view' but this expression writes to state"));

    let file = r#"
    contract testTypes {
    function testAsm() public view {
        assembly {
            {
                for {let i := 6} gt(i, 0) {} {
                    i := add(selfbalance(), i)
                }
            }
        }
    }
}    "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
}

#[test]
fn function_without_modifier() {
    let file = r#"
    contract testTypes {
    function testAsm() public {
        assembly {
            {
                for {let i := caller()} gt(i, 0) {} {
                    let x := add(i, 2)
                }
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 3);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'x' has never been read"));
    assert!(ns
        .diagnostics
        .contains_message("function can be declared 'view'"));
}
