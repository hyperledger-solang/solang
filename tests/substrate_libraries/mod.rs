extern crate solang;
use super::{build_solidity, first_error, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::Target;

#[test]
fn restrictions() {
    let ns = parse_and_resolve(
        r#"
        library c {        
            constructor() {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "constructor not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            receive() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "receive not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            fallback() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "fallback not allowed in a library"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            function f() public payable {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function in a library cannot be payable"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            function foo() virtual public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "functions in a library cannot be virtual"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            function foo() override public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function in a library cannot override"
    );

    let ns = parse_and_resolve(
        r#"
        library c is x {        
            fallback() internal {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library ‘c’ cannot have a base contract"
    );

    let ns = parse_and_resolve(
        r#"
        library c {        
            function foo() public { }
        }
        
        contract a is c {        
            function bar() public { }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "library ‘c’ cannot be used as base contract for contract ‘a’"
    );
}

#[test]
fn simple() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let mut runtime = build_solidity(
        r##"
        contract test {
            function foo(uint64 x) public pure returns (uint64) {
                return ints.max(x, 65536);
            }
        }

        library ints {
            function max(uint64 a, uint64 b) private pure returns (uint64) {
                return a > b ? a : b;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("foo", Val(102).encode());

    assert_eq!(runtime.vm.scratch, Val(65536).encode());
}
