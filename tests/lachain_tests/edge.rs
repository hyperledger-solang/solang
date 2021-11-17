use crate::{no_errors, parse_and_resolve, build_solidity};
use solang::Target;
use ethabi::Token;
use ethereum_types::Address;
use std::str::FromStr;

#[test]
fn ternary() {
    let ns = parse_and_resolve(
        r#"
        contract Test {
            function test(bool _b) public {
                uint24 n1 = 1;
                uint112 n2 = 2;
                uint256 r = (_b == true ? n1 : n2);
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn multiple_inheritence() {
    let ns = parse_and_resolve(
        r#"
        interface IA {
            function test() external;
        }
        
        interface IB is
            IA
        {
            
        }
        
        contract C {
            function test(address _a) public {
                IB(_a).test();
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn tuple_return() {
    let ns = parse_and_resolve(
        r#"
        contract A {
            function testA() public returns (uint256, uint256) {
               return (0,0);
            }
        
            function testB() public returns (uint256, uint256) {
               return testA();
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn tuple_assignment() {
    let ns = parse_and_resolve(
        r#"
        contract A {
            struct structA {
                uint256 value;
            }
        
            function testA() public returns (uint256, uint256) {
               return (0, 0);
            }
        
            function testB() public {
               structA memory _structA;
        
               (_structA.value, ) = testA();
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn assembly() {
    let ns = parse_and_resolve(
        r#"
        contract A {
            function test() public {
               assembly {
                   z := add(1, 5)
               }
            }
        }
        "#,
        Target::Lachain,
    );

    no_errors(ns.diagnostics);
}

#[test]
fn abi_virtual_override() {
    let mut vm = build_solidity(
        r#"
        abstract contract A {
            function test() virtual public returns (bool)  {
               return true;
            }
        }
        
        contract B is A {
            function test() override public returns (bool) {
                return false;
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("test", &[]);

    assert_eq!(returns, vec![ethabi::Token::Bool(false),]);
}

#[test]
fn abi_encode_packed() {
    let mut vm = build_solidity(
        r#"
        contract foo {
            function test() public returns (bytes) {
                return abi.encodePacked(true, false);
            }
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("test", &[]);

    let bytes = vec![1, 0];

    assert_eq!(returns, vec![Token::Bytes(bytes)]);
}

#[test]
fn address_literal() {
    let mut vm = build_solidity(
        r#"
        contract B {
            address public constant a = 0x45C978D685D3B781e2D67642108c42813205c0E4;
        }"#,
    );

    vm.constructor(&[]);

    let returns = vm.function("a", &[]);

    assert_eq!(
        returns,
        vec![ethabi::Token::Address(
            Address::from_str("0x45C978D685D3B781e2D67642108c42813205c0E4").unwrap()
        )]
    );
}

#[test]
#[should_panic]
fn require_false_reason() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(false, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
}

#[test]
fn require_true_reason() {
    // without a working assert, this is not going to work
    let mut vm = build_solidity(
        r#"
        contract foo {
            function assert_fails() public {
                require(true, "humpty-dumpty");
            }
        }"#,
    );

    vm.constructor(&[]);

    vm.function("assert_fails", &[]);
}
