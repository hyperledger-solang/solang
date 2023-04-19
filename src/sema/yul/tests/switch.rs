// SPDX-License-Identifier: Apache-2.0

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
        r#"unrecognised token 'case', expected "abstract", "address", "anonymous", "as", "assembly", "bool", "break", "byte", "bytes", "calldata", "catch", "constant", "constructor", "continue", "contract", "do", "else", "emit", "enum", "event", "external", "fallback", "for", "function", "if", "immutable", "import", "indexed", "interface", "internal", "is", "leave", "let", "library", "mapping", "memory", "modifier", "new", "override", "payable", "pragma", "private", "public", "pure", "receive", "return", "returns", "revert", "storage", "string", "struct", "switch", "throw", "try", "unchecked", "using", "view", "virtual", "while", "{", "}", Int, Uint, identifier"#
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
        r#"unrecognised token 'default', expected "abstract", "address", "anonymous", "as", "assembly", "bool", "break", "byte", "bytes", "calldata", "catch", "constant", "constructor", "continue", "contract", "do", "else", "emit", "enum", "event", "external", "fallback", "for", "function", "if", "immutable", "import", "indexed", "interface", "internal", "is", "leave", "let", "library", "mapping", "memory", "modifier", "new", "override", "payable", "pragma", "private", "public", "pure", "receive", "return", "returns", "revert", "storage", "string", "struct", "switch", "throw", "try", "unchecked", "using", "view", "virtual", "while", "{", "}", Int, Uint, identifier"#
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
    assert_eq!(ns.diagnostics.len(), 1);
    assert_eq!(
        ns.diagnostics.iter().next().unwrap().message,
        "found contract 'testTypes'"
    );
}

#[test]
fn repeated_switch_case() {
    let file = r#"
contract Testing {
    function duplicate_cases(uint a) public pure returns (uint b) {
        assembly {
            switch a
            case hex"019a" {
                b := 5
            }
            case 410 {
                b := 6
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns.diagnostics.contains_message("found contract 'Testing'"));
    assert!(ns.diagnostics.contains_message("duplicate case for switch"));
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "repeated case found here");

    let file = r#"
contract Testing {
    function duplicate_cases(uint a) public pure returns (uint b) {
        assembly {
            switch a
            case true {
                b := 5
            }
            case 1 {
                b := 6
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns.diagnostics.contains_message("found contract 'Testing'"));
    assert!(ns.diagnostics.contains_message("duplicate case for switch"));
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "repeated case found here");

    let file = r#"
contract Testing {
    function duplicate_cases(uint a) public pure returns (uint b) {
        assembly {
            switch a
            case 0 {
                b := 5
            }
            case false {
                b := 6
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns.diagnostics.contains_message("found contract 'Testing'"));
    assert!(ns.diagnostics.contains_message("duplicate case for switch"));
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "repeated case found here");

    let file = r#"
contract Testing {
    function duplicate_cases(uint a) public pure returns (uint b) {
        assembly {
            switch a
            case 16705 {
                b := 5
            }
            case "AA" {
                b := 6
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns.diagnostics.contains_message("found contract 'Testing'"));
    assert!(ns.diagnostics.contains_message("duplicate case for switch"));
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "repeated case found here");

    let file = r#"
contract Testing {
    function duplicate_cases(uint a) public pure returns (uint b) {
        assembly {
            switch a
            case 16705 {
                b := 5
            }
            case 16705 {
                b := 6
            }
        }
    }
}
    "#;
    let ns = parse(file);
    assert_eq!(ns.diagnostics.len(), 2);
    assert!(ns.diagnostics.contains_message("found contract 'Testing'"));
    assert!(ns.diagnostics.contains_message("duplicate case for switch"));
    let errors = ns.diagnostics.errors();
    assert_eq!(errors.len(), 1);
    assert_eq!(errors[0].notes.len(), 1);
    assert_eq!(errors[0].notes[0].message, "repeated case found here");
}
