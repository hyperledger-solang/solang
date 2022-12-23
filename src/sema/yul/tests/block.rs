// SPDX-License-Identifier: Apache-2.0

use crate::sema::yul::tests::parse;

#[test]
fn unreachable_leave() {
    let file = r#"
    contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                function tryThis(a, b) {
                    a := add(a, 4)
                    leave
                    b := add(b, 6)
                    let c := mul(a, b)
                }
            }
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
            {
                function tryThis(a, b) {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    b := add(b, 6)
                    let c := mul(a, b)
                }
            }
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
        .contains_message("yul function has never been used"));
}

#[test]
fn unreachable_continue() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                let a := 0
                for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                    a := shr(a, 2)
                    if lt(a, 4) {
                        continue
                        let b := shr(6, 5)
                        a := mul(a, b)
                    }
                }
            }
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
            {
                let a := 0
                for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                    a := shr(a, 2)
                    if lt(a, 4) {
                        continue
                    }
                    let b := shr(6, 5)
                        a := mul(a, b)
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
fn unreachable_break() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            {
                let a := 0
                for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                    a := shr(a, 2)
                    if lt(a, 4) {
                        break
                        let b := shr(6, 5)
                        a := mul(a, b)
                    }
                }
            }
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
            {
                let a := 0
                for {let i := 0} lt(i, 10) {i := add(i, 1)} {
                    a := shr(a, 2)
                    if lt(a, 4) {
                        break
                    }
                    let b := shr(6, 5)
                        a := mul(a, b)
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
fn unreachable_switch() {
    let file = r#"
contract testTypes {
    function testAsm() public {
        assembly {
            {
                let a := 0
                for {let i := 11} lt(i, 10) {i := add(i, 1)} {
                    a := shr(a, 2)
                    if lt(a, 4) {
                        continue
                    }
                    let b := shr(6, 5)
                        a := mul(a, b)
                    stop()
                }

                switch a
                case 0 {stop()}
                case 1 {return(0, 0)}
                case 2 {revert(2, 3)}
                case 3 {selfdestruct(0x40)}
                default {invalid()}

                let b := shr(a, 1)
                return(b, 2)
            }
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
            {
                let a := 0

                switch a
                case 0 {stop()}
                case 1 {return(0, 0)}
                case 2 {revert(2, 3)}
                case 3 {selfdestruct(0x40)}
                default {a := add(a, 3)}

                let b := shr(a, 1)
                return(b, 2)
            }
        }
    }
}    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 1);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));

    let file = r#"
    contract testTypes {
    function testAsm() public {
        assembly {
            {
                let a := 0

                switch a
                case 0 {stop()}
                case 1 {return(0, 0)}
                case 2 {revert(2, 3)}
                case 3 {selfdestruct(0x40)}

                let b := shr(a, 1)
                return(b, 2)
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
fn unreachable_after_leave() {
    let file = r#"
contract testTypes {
    function testAsm() pure public {
        assembly {
            {
                function tryThis(b, a) -> c {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    b := add(a, 6)
                    c := tryThat(b, 2)
                    invalid()
                }

                {
                    function foo(d) -> e {
                        e := shr(d, 3)
                    }

                    let y := tryThis(foo(3), 2)
                    invalid()
                }

                function tryThat(b, a) -> c {
                    a := add(a, 4)
                    if gt(a, 5) {
                        leave
                    }
                    c := 5
                    invalid()
                }
                let x := 5
            }
        }
    }
}   "#;

    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 4);
    assert!(ns
        .diagnostics
        .contains_message("found contract 'testTypes'"));
    assert!(ns.diagnostics.contains_message("unreachable yul statement"));
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'x' has never been read"));
    assert!(ns
        .diagnostics
        .contains_message("yul variable 'y' has never been read"));
}
