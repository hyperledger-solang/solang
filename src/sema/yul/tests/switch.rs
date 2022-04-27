#![cfg(test)]

use crate::sema::yul::tests::parse;

#[test]
fn case_not_literal() {
    let file = r#"
    contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(7, 6)
            let y : u32 := 0x90
            switch x
            case 0 { revert(0, 0)}
            case y { mstore(0, 0x40)}
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message(
        r#"unrecognised token 'y', expected "false", "true", hexnumber, hexstring, number, string"#
    ));
}

#[test]
fn case_after_default() {
    let file = r#"
contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(7, 6)
            let y : u32 := 0x90
            switch x
            case 0 { revert(0, 0)}
            default { mstore(0, 0x40)}
            case 2 { x := shr(y, 2)}
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message(
        r#"unrecognised token 'case', expected "address", "bool", "break", "byte", "continue", "for", "function", "if", "leave", "let", "return", "revert", "switch", "{", "}", identifier"#
    ));
}

#[test]
fn double_default() {
    let file = r#"
contract testTypes {
    function testAsm() public view {
        assembly {
            let x := add(7, 6)
            let y : u32 := 0x90
            switch x
            case 0 { revert(0, 0)}
            default { mstore(0, 0x40)}
            default { x := shr(y, 2)}
        }
    }
}
    "#;

    let ns = parse(file);
    assert!(ns.diagnostics.contains_message(
        r#"unrecognised token 'default', expected "address", "bool", "break", "byte", "continue", "for", "function", "if", "leave", "let", "return", "revert", "switch", "{", "}", identifier"#
    ));
}

#[test]
fn correct_switch() {
    let file = r#"
contract testTypes {
    function testAsm() public pure {
        assembly {
            function power(base, exponent) -> result {
                switch exponent
                case 0 {
                    result := 1
                }
                case 1 {
                    result := base
                }
                default {
                    result := power(mul(base, base), div(exponent, 2))
                    switch mod(exponent, 2)
                    case 1 {
                        result := mul(base, result)
                    }
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
        .contains_message("inline assembly is not yet supported"));
}
