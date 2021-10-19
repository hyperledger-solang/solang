use crate::{build_solidity, first_error, first_warning, no_errors, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::file_resolver::FileResolver;
use solang::Target;

#[test]
fn test_virtual() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            function test() public;
        }"#,
        Target::default_substrate(),
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
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract c {
            function test() virtual public;
            function test2() virtual public;
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract should be marked ‘abstract contract’ since it has 2 functions with no body"
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
        Target::default_substrate(),
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
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "cannot construct ‘foo’ of type ‘abstract contract’"
    );

    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
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
        Target::default_substrate(),
        false,
    );

    no_errors(ns.diagnostics);

    assert_eq!(contracts.len(), 1);

    let mut cache = FileResolver::new();

    cache.set_file_contents(
        "a.sol",
        r#"
        contract foo {
            function f1() public {
            }
        }"#
        .to_string(),
    );

    cache.set_file_contents(
        "b.sol",
        r#"
        import "a.sol";

        contract bar is foo {
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
        Target::default_substrate(),
        false,
    );

    no_errors(ns.diagnostics);

    assert_eq!(contracts.len(), 1);
}

#[test]
fn test_interface() {
    let ns = parse_and_resolve(
        r#"
        interface foo {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "constructor not allowed in an interface"
    );

    let ns = parse_and_resolve(
        r#"
        interface foo {
            function bar() external {}
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function in an interface cannot have a body"
    );

    let ns = parse_and_resolve(
        r#"
        interface foo {
            function bar() private;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "functions must be declared ‘external’ in an interface"
    );

    let ns = parse_and_resolve(
        r#"
        interface foo {
            function bar() internal;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "functions must be declared ‘external’ in an interface"
    );

    let ns = parse_and_resolve(
        r#"
        interface foo is a {
            function bar() internal;
        }

        abstract contract a {
            function f() internal {}
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "interface ‘foo’ cannot have abstract contract ‘a’ as a base"
    );

    let ns = parse_and_resolve(
        r#"
        interface foo is a {
            function bar() internal;
        }

        contract a {
            function f() internal {}
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "interface ‘foo’ cannot have contract ‘a’ as a base"
    );

    let ns = parse_and_resolve(
        r#"
        interface bar {
            function foo() virtual external;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "functions in an interface are implicitly virtual"
    );

    let ns = parse_and_resolve(
        r#"
        interface bar {
            int x;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "interface ‘bar’ is not allowed to have contract variable ‘x’"
    );

    let ns = parse_and_resolve(
        r#"
        interface bar {
            int constant x = 1;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "interface ‘bar’ is not allowed to have contract variable ‘x’"
    );

    // 1. implementing an interface does not require an override

    // 2. interface is function implemented by base
    let ns = parse_and_resolve(
        r#"
        interface bar {
            function f1(address a) external;
        }

        interface bar2 {
            function f1(address a) external;
        }

        contract x is bar {
            function f1(address a) public {}
        }

        contract y is bar2, x {
            function f2(address a) public {}
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);
}

#[test]
fn inherit() {
    let ns = parse_and_resolve(
        r#"
        contract a is a {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘a’ cannot have itself as a base contract"
    );

    let ns = parse_and_resolve(
        r#"
        contract a is foo {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "contract ‘foo’ not found");

    let ns = parse_and_resolve(
        r#"
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is a {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "base ‘a’ from contract ‘b’ is cyclic"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            constructor(int arg1) public {
            }
        }

        contract b is a, a {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘b’ duplicate base ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is c {
            constructor(int arg1) public {
            }
        }

        contract c is a {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "base ‘a’ from contract ‘c’ is cyclic"
    );

    let ns = parse_and_resolve(
        r#"
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is c {
            constructor(int arg1) public {
            }
        }

        contract d {
            constructor(int arg1) public {
            }
        }

        contract c is d, a {
            constructor(int arg1) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "base ‘a’ from contract ‘c’ is cyclic"
    );
}

#[test]
fn inherit_types() {
    let ns = parse_and_resolve(
        r#"
        contract a is b {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }
        }

        contract b {
            enum enum_x { x1, x2 }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract a is b {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }

            function test2() public returns (enum_y) {
                return enum_y.y2;
            }
        }

        contract b is c {
            enum enum_y { y1, y2 }
        }

        contract c {
            enum enum_x { x1, x2 }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract a is b, c {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }

            function test2() public returns (enum_y) {
                return enum_y.y2;
            }
        }

        contract b is c {
            enum enum_y { y1, y2 }
        }

        contract c {
            enum enum_x { x1, x2 }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract a {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }
        }

        contract b {
            enum enum_x { x1, x2 }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "type ‘enum_x’ not found");

    let ns = parse_and_resolve(
        r#"
        contract a is b {
            foo public var1;
        }

        contract b {
            struct foo {
                uint32 f1;
                uint32 f2;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract b {
            struct foo {
                uint32 f1;
                uint32 f2;
            }
        }

        contract c {
            enum foo { f1, f2 }
        }

        contract a is b, c {
            function test(foo x) public {
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "already defined ‘foo’");
}

#[test]
fn inherit_variables() {
    let ns = parse_and_resolve(
        r#"
        contract b {
            int foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract b {
            int private foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "`foo\' is not found");

    let ns = parse_and_resolve(
        r#"
        contract a {
            int public foo;
        }

        contract b is a {
            int public bar;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract a {
            int private foo;
        }

        contract b is a {
            int public foo;
        }

        contract c is b {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract a {
            int public constant foo = 0xbffe;
        }

        contract c is a {
            function getFoo() public returns (int) {
                return foo;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            uint16 public foo = 65535;
        }

        contract a {
            uint16 private foo = 102;
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(102, 0)
    );

    slot[0] = 1;

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(0xff, 0xff)
    );

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            uint16 public var_b;

            function test() public {
                var_a = 102;
                var_b = 65535;
            }
        }

        contract a {
            uint16 public var_a;
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());

    let mut slot = [0u8; 32];

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(102, 0)
    );

    slot[0] = 1;

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(0xff, 0xff)
    );
}

#[test]
fn call_inherited_function() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let mut runtime = build_solidity(
        r##"
        contract apex is base {
            function bar() public returns (uint64) {
                return foo() + 3;
            }
        }

        contract base {
            function foo() public returns (uint64) {
                return 102;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val(105).encode());

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo() private returns (uint64) {
                return 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo() + 3;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "cannot call private function");

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }

        contract apex is base {
            function bar() public returns (uint64) {
                return foo({a: 3}) + 3;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "cannot call private function");

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) public returns (uint64) {
                return a + 102;
            }
        }

        contract apex is base {
            function foo(uint64 a) public returns (uint64) {
                return a + 64;
            }

            function bar() public returns (uint64) {
                return foo({a: 3}) + 3;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ with this signature already defined"
    );

    let mut runtime = build_solidity(
        r##"
        contract apex is base {
            uint64 private x = 7;

            function bar() public returns (uint64) {
                return foo() + x + 13;
            }
        }

        contract base {
            uint64 private x = 5;

            function foo() public returns (uint64) {
                return x + 11;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val(36).encode());

    let mut runtime = build_solidity(
        r##"
        contract apex is base, base2 {
            uint64 private x = 7;

            function bar() public returns (uint64) {
                return foo() + foo2() + x + 13;
            }
        }

        contract base {
            uint64 private x = 50000;

            function foo() public returns (uint64) {
                return x + 110000;
            }
        }

        contract base2 {
            uint64 private x = 600;

            function foo2() public returns (uint64) {
                return x + 1100;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val(161720).encode());

    let mut runtime = build_solidity(
        r##"
        contract apex is base, base2 {
            function foo(int64 x) public returns (uint64) {
                return 3;
            }
        }

        contract base {
            function foo() public returns (uint64) {
                return 1;
            }
        }

        contract base2 {
            function foo(bool) public returns (uint64) {
                return 2;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());
    assert_eq!(runtime.vm.output, Val(1).encode());

    runtime.raw_function([0x45, 0x55, 0x75, 0x78, 1].to_vec());
    assert_eq!(runtime.vm.output, Val(2).encode());

    runtime.raw_function([0x36, 0x8E, 0x4A, 0x7F, 1, 2, 3, 4, 5, 6, 7, 8].to_vec());
    assert_eq!(runtime.vm.output, Val(3).encode());
}

#[test]
fn test_override() {
    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) override override private returns (uint64) {
                return a + 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function redeclared ‘override’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘bar’ in override list not found"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function f() private {}
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "override ‘bar’ is not a base contract of ‘base’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            function foo(uint64 a) override private returns (uint64) {
                return a + 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ does not override anything"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar {
            function foo(uint64 a) override(bar) private returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) private returns (uint64) {
                return a + 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ overrides function which is not virtual"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            uint64 public x;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ override list does not contain ‘bar’"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            int64 public x = 3;
            function f() virtual payable external {
                x = 1;
            }

            function f() override payable external {
                x = 2;
            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘f’ overrides function in same contract"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            function foo() virtual public returns (int32) {
                return 1;
            }
        }

        contract b is a {
            function foo() virtual override public returns (int32) {
                return 2;
            }
        }

        contract c is b {
            function foo() override public returns (int32) {
                return 3;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            receive() override payable external {
                x = 2;
            }
        }

        contract a {
            int8 public x = 3;
            receive() virtual payable external {
                x = 1;
            }
        }

        contract c is b {
            function test() public returns (int8) {
                return x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];
    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(3)
    );

    runtime.vm.value = 1;
    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());

    let slot = [0u8; 32];

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(2)
    );

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            fallback() override external {
                x = 2;
            }
        }

        contract a {
            int8 public x = 3;
            fallback() virtual external {
                x = 1;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];
    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(3)
    );

    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());

    let slot = [0u8; 32];

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(2)
    );

    let ns = parse_and_resolve(
        r#"
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int x) public { print ("foo"); }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘a’ missing override for function ‘bar’"
    );

    let ns = parse_and_resolve(
        r#"
        interface b {
                function bar(int64 x) external;
        }

        contract a is b {
                function bar(int64 x) public override;
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function with no body must be marked ‘virtual’"
    );
}

#[test]
fn multiple_override() {
    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) override internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ should specify override list ‘override(bar2,bar)’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ missing overrides ‘bar2’, specify ‘override(bar2,bar)’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2, bar3 {
            function foo(uint64 a) override(bar,bar2,bar3) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }

        contract bar3 {
            function f() public {

            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ includes extraneous overrides ‘bar3’, specify ‘override(bar2,bar)’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ overrides functions which are not ‘virtual’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ should specify override list ‘override(bar2,bar)’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 104;
            }

            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ should specify override list ‘override(bar2,bar)’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base is bar, bar2 {
            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 104;
            }

            function foo(uint64 a) override(bar,bar2) internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }

        contract bar2 {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 103;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ overrides function in same contract"
    );
}

#[test]
fn base_contract() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u32);

    let ns = parse_and_resolve(
        r#"
        contract base {
            constructor(uint64 a) {}
        }

        contract apex is base {
            constructor() {}
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "missing arguments to contract ‘base’ constructor"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            constructor(uint64 a) public {}
        }

        contract apex is base(true) {
            function foo(uint64 a) virtual internal returns (uint64) {
                return a + 102;
            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from bool to uint64 not possible"
    );

    let mut runtime = build_solidity(
        r##"
        contract b is a(foo) {
            int32 constant foo = 102;

            function f() public returns (int32) {
                    return bar;
            }
        }

        contract a {
                int32 public bar;
                constructor(int32 x) public {
                        bar = x;
                }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("f", Vec::new());

    assert_eq!(runtime.vm.output, Val(102).encode());
}

#[test]
fn base_contract_on_constructor() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    let ns = parse_and_resolve(
        r#"
        contract base {
            struct s { uint32 f1; }
        }

        contract b {
            struct s { uint32 f1; }
        }

        contract apex is base {
            constructor() public b {

            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘b’ is not a base contract of ‘apex’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            struct s { uint32 f1; }
        }

        contract b {
            struct s { uint32 f1; }
        }

        contract apex is base {
            constructor() public b {

            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘b’ is not a base contract of ‘apex’"
    );

    let ns = parse_and_resolve(
        r#"
        contract apex {
            constructor() public apex {

            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "contract ‘apex’ is not a base contract of ‘apex’"
    );

    let ns = parse_and_resolve(
        r#"
        contract apex {
            constructor() oosda public {

            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unknown function attribute ‘oosda’"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            constructor(bool x) {}
        }

        contract apex is base {
                function foo() pure public {}
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "missing arguments to base contract ‘base’ constructor"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            constructor(bool x) {}
        }

        contract apex is base {
            constructor() base(true) base(false) {}
            function foo() pure public {}
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate base contract ‘base’"
    );

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            int32 constant BAR = 102;
            int64 public foo;

            constructor(int64 i) a(BAR) { foo = i; }

            function get_x() public returns (int32) {
                    return x;
            }
        }

        contract a {
                int32 public x;

                constructor(int32 i) { x = i; }
        }"##,
    );

    runtime.constructor(0, Val64(0xbffe).encode());
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Val(102).encode());

    let mut runtime = build_solidity(
        r##"
        contract c is b(2) {
            constructor() {
            }
        }

        contract a {
                int32 public x;

                constructor(int32 i) { x = i; }
        }

        contract b is a {
                int32 constant BAR = 102;
                int64 public foo;

                constructor(int64 i) a(BAR + int32(i)) { foo = i; }

                function get_x() public view returns (int32) {
                        return x;
                }

                function get_foo() public view returns (int64) {
                        return foo;
                }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Val(104).encode());

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            constructor(int64 x) b(x+3) {}
        }

        contract b is a {
            constructor(int64 y) a(y+2) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }"##,
    );

    runtime.constructor(0, Val64(7).encode());
    runtime.function("get_foo", Vec::new());

    assert_eq!(runtime.vm.output, Val64(12).encode());

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            constructor(int64 x) b(x+3) a(x+5){}
        }

        abstract contract b is a {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }"##,
    );

    runtime.constructor(0, Val64(7).encode());
    runtime.function("get_foo", Vec::new());

    assert_eq!(runtime.vm.output, Val64(12).encode());

    let ns = parse_and_resolve(
        r##"
        contract c is b {
            constructor(int64 x) b(x+3) {}
        }

        abstract contract b is a {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "missing arguments to base contract ‘a’ constructor"
    );

    let ns = parse_and_resolve(
        r##"
        contract c is b {
            constructor(int64 x) b(x+3) b(0) {}
        }

        abstract contract b is a {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "duplicate base contract ‘b’");

    let ns = parse_and_resolve(
        r##"
        contract c is b {
            constructor(int64 x) b(x+3) a(0) {}
        }

        abstract contract b is a(2) {
            constructor(int64 y) {}
        }

        contract a {
            int64 foo;
            function get_foo() public returns (int64) { return foo; }
            constructor(int64 z) { foo = z; }
        }"##,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate argument for base contract ‘a’"
    );
}

#[test]
fn call_base_function_via_basename() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(u64);

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            function bar() public returns (uint64) {
                return a.foo();
            }
        }

        contract b is a {
            function foo() internal override returns (uint64) {
                return 2;
            }
        }

        contract a {
            function foo() internal virtual returns (uint64) {
                return 1;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val64(1).encode());

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            uint64 constant private C = 100;
            function bar() public returns (uint64) {
                return a.foo({ x: C });
            }
        }

        contract b is a {
            uint64 constant private C = 300;
            function foo(uint64 x) internal override returns (uint64) {
                return 2;
            }
        }

        contract a {
            uint64 constant private C = 200;
            function foo(uint64 x) internal virtual returns (uint64) {
                return 1 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, Val64(101).encode());
}

#[test]
fn simple_interface() {
    let ns = parse_and_resolve(
        r#"
        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }

        contract foo is IFoo {
            function bar(uint32 a) public pure returns (uint32) {
                return a * 2;
            }
        }"#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let mut runtime = build_solidity(
        r##"
        contract foo is IFoo {
            function bar(uint32 a) public pure override returns (uint32) {
                return a * 2;
            }
        }

        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", 100u32.encode());

    assert_eq!(runtime.vm.output, 200u32.encode());
}

#[test]
fn cast_contract() {
    let ns = parse_and_resolve(
        r#"
        interface operator {
            function op1(int32 a, int32 b) external returns (int32);
            function op2(int32 a, int32 b) external returns (int32);
        }

        contract ferqu {
            operator op;

            constructor(bool do_adds) {
                if (do_adds) {
                    op = new m1();
                } else {
                    op = new m2();
                }
            }

            function x(int32 b) public returns (int32) {
                return op.op1(102, b);
            }
        }

        contract m1 is operator {
            function op1(int32 a, int32 b) public override returns (int32) {
                return a + b;
            }

            function op2(int32 a, int32 b) public override returns (int32) {
                return a - b;
            }
        }

        contract m2 is operator {
            function op1(int32 a, int32 b) public override returns (int32) {
                return a * b;
            }

            function op2(int32 a, int32 b) public override returns (int32) {
                return a / b;
            }
        }"#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        interface IFoo {
            function bar(uint32) external pure returns (uint32);
        }

        contract foo  {
            function bar(IFoo x) public pure returns (uint32) {
                foo y = x;
            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion not allowed since contract foo is not a base contract of contract IFoo"
    );
}

#[test]
fn test_super() {
    let ns = parse_and_resolve(r#"contract super {}"#, Target::default_substrate());

    assert_eq!(
        first_error(ns.diagnostics),
        "‘super’ shadows name of a builtin"
    );

    let ns = parse_and_resolve(
        r#"
        function f1() { super.a(); }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "super not available outside contracts"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            function f1() public {}
        }

        contract b is a {
            function f2() public {
                super.f2();
            }
        }"#,
        Target::default_substrate(),
    );

    assert_eq!(first_error(ns.diagnostics), "unknown function or type ‘f2’");

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            function bar() public returns (uint64) {
                super.foo();

                return var;
            }

            function foo() internal override {
                var = 103;
            }
        }

        contract a {
            uint64 var;

            function foo() internal virtual {
                var = 102;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, 102u64.encode());

    let mut runtime = build_solidity(
        r##"
        contract b is a {
            function bar() public returns (uint64) {
                super.foo({x: 10});

                return var;
            }

            function foo2(uint64 x) internal {
                var = 103 + x;
            }
        }

        contract a {
            uint64 var;

            function foo(uint64 x) internal virtual {
                var = 102 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, 112u64.encode());

    let mut runtime = build_solidity(
        r##"
        contract b is a, aa {
            function bar() public returns (uint64) {
                return super.foo({x: 10});
            }

            function foo(uint64 x) public override(a, aa) returns (uint64) {
                return 103 + x;
            }
        }

        contract a {
            function foo(uint64 x) public virtual returns (uint64) {
                return 102 + x;
            }
        }

        contract aa {
            function foo(uint64 x) public virtual returns (uint64) {
                return 202 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.vm.output, 112u64.encode());
}

#[test]
fn mutability() {
    let ns = parse_and_resolve(
        r#"
        contract y {
            function foo() external pure virtual returns (int) {
                return 102;
            }
        }

        contract x is y {
            function foo() external override returns (int) {
                return 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "mutability ‘nonpayable’ of function ‘foo’ is not compatible with mutability ‘pure’"
    );

    let ns = parse_and_resolve(
        r#"
        abstract contract y {
            function foo() external view virtual returns (int);
        }

        contract x is y {
            function foo() external payable override returns (int) {
                return 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "mutability ‘payable’ of function ‘foo’ is not compatible with mutability ‘view’"
    );
}

#[test]
fn visibility() {
    let ns = parse_and_resolve(
        r#"
        contract y {
            function foo() external virtual returns (int) {
                return 102;
            }
        }

        contract x is y {
            function foo() public override returns (int) {
                return 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        abstract contract y {
            function foo() external virtual returns (int);
        }

        contract x is y {
            function foo() internal override returns (int) {
                return 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "visibility ‘internal’ of function ‘foo’ is not compatible with visibility ‘external’"
    );

    let ns = parse_and_resolve(
        r#"
        abstract contract y {
            function foo() internal virtual returns (int);
        }

        contract x is y {
            function foo() private override returns (int) {
                return 102;
            }
        }
        "#,
        Target::default_substrate(),
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "visibility ‘private’ of function ‘foo’ is not compatible with visibility ‘internal’"
    );
}

#[test]
fn var_or_function() {
    let mut runtime = build_solidity(
        r##"
        contract x is c {
            function f1() public returns (int64) {
                return c.selector();
            }

            function f2() public returns (int64)  {
                function() internal returns (int64) a = c.selector;
                return a();
            }
        }

        contract c {
            int64 public selector = 102;
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("f1", Vec::new());

    assert_eq!(runtime.vm.output, 102u64.encode());

    runtime.function("f2", Vec::new());

    assert_eq!(runtime.vm.output, 102u64.encode());
}
