use crate::sema::assembly::tests::{assert_message_in_diagnostics, parse};

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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable assembly statement"
    ));

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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable assembly statement"
    ));

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
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable assembly statement"
    ));

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
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
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
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "unreachable assembly statement"
    ));

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
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));

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
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "found contract ‘testTypes’"
    ));
    assert!(assert_message_in_diagnostics(
        &ns.diagnostics,
        "evm assembly not supported on target solana"
    ));
}
