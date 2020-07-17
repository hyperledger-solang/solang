extern crate solang;

use super::{first_error, no_errors, parse_and_resolve};
use solang::file_cache::FileCache;
use solang::Target;

#[test]
fn test_virtual() {
    let ns = parse_and_resolve(
        r#"
        contract c {        
            function test() public;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function with no body must be marked ‘virtual’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {        
            function test() virtual public {}
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function marked ‘virtual’ cannot have a body"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function test() virtual public;
            function test2() virtual public;
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract should be marked ‘abstract contract’ since it has 2 virtual functions"
    );
}

#[test]
fn test_abstract() {
    let ns = parse_and_resolve(
        r#"
        abstract contract foo {
            constructor(int arg1) public {
            }
        
            function f1() public {
            }
        }
        
        contract bar {
            function test() public {
                foo x = new foo(1);
            }
        }
        "#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot construct ‘foo’ of type ‘abstract contract’"
    );

    let ns = parse_and_resolve(
        r#"
        abstract contract foo {
            constructor(int arg1) public {
            }
        
            function f1() public {
            }
        }
        
        contract bar {
            function test() public {
                foo x = new foo({arg: 1});
            }
        }
        "#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot construct ‘foo’ of type ‘abstract contract’"
    );

    let mut cache = FileCache::new();

    cache.set_file_contents(
        "a.sol".to_string(),
        r#"
        abstract contract foo {
            constructor(int arg1) public {
            }
        
            function f1() public {
            }
        }
        
        contract bar {
            function test() public returns (uint32) {
                return 102;
            }
        }
        "#
        .to_string(),
    );

    let (contracts, ns) = solang::compile(
        "a.sol",
        &mut cache,
        inkwell::OptimizationLevel::Default,
        Target::Substrate,
    );

    no_errors(ns.diagnostics);

    assert_eq!(contracts.len(), 1);
}
