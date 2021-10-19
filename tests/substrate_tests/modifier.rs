use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};
use solang::Target;

#[test]
fn declare() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() public {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘public’: modifiers can not have visibility"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() internal {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "‘internal’: modifiers can not have visibility"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() payable {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "modifier cannot have mutability specifier"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() pure {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "modifier cannot have mutability specifier"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo bar {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function modifiers or base contracts are not allowed on modifier"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(first_error(ns.diagnostics), "missing ‘_’ in modifier");

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo(bool x) {
                if (true) {
                    while (x) {
                        _;
                    }
                }
            }
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() {
                _;
            }

            function bar() public {
                foo();
            }
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "unknown function or type ‘foo’"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            function bar() public {
                _;
            }
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "underscore statement only permitted in modifiers"
    );
}

#[test]
fn function_modifier() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() { _; }

            function bar() foo2 public {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(first_error(ns.diagnostics), "unknown modifier ‘foo2’");

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo() { _; }

            function bar() foo(1) public {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "modifier expects 0 arguments, 1 provided"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            modifier foo(int32 f) { _; }

            function bar(bool x) foo(x) public {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from bool to int32 not possible"
    );
}

#[test]
fn chain() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            uint16 public var;

            modifier foo() {
                bool boom = true;
                if (boom) {
                    _;
                }
            }

            function bar() foo() public {
                var = 7;
            }
        }"#,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("bar", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(7, 0)
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1 {
                var = 3;
                _;
                var = 5;
            }

            function test() mod1 public {
                assert(var == 3);
                var = 7;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(5, 0)
    );

    // now test modifier with argument and test that function argument is passed on
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1(uint16 v) {
                var = 3;
                _;
                assert(var == 11);
                var = v;
            }

            function test(uint16 x) mod1(x - 6) public {
                assert(var == 3);
                var = x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", 11u16.encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(5, 0)
    );

    // now test modifier with argument and test that function argument is passed on
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier mod1(uint16 v) {
                assert(var == 0);

                var = 3;
                _;
                assert(var == 17);
                var = v;
            }

            modifier mod2(uint16 v) {
                assert(var == 3);
                var = 9;
                _;
                assert(var == 11);
                var = v;
            }

            function test(uint16 x) mod1(x - 6) mod2(x + 6) public {
                assert(var == 9);
                var = x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", 11u16.encode());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(5, 0)
    );

    // two placeholders means the following function is called twice.
    let mut runtime = build_solidity(
        r##"
        contract c {
            uint16 public var;

            modifier m {
                _;
                _;
            }

            function test() m public {
                var += 3;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let slot = [0u8; 32];

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(6, 0)
    );
}

#[test]
fn inherit_modifier() {
    let mut runtime = build_solidity(
        r##"
        contract c is base {
            function test() md2 public {
                    assert(s2 == 2);
                    s2 += 3;
            }
        }

        contract base {
                bool private s1;
                int32 internal s2;

                modifier md2 {
                        s2 += 2;
                        _;
                        s2 += 2;
                }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];
    slot[0] = 1;

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(7, 0, 0, 0)
    );

    // now override it
    let mut runtime = build_solidity(
        r##"
        contract c is base {
            function test() md2 public {
                    assert(s2 == 2);
                    s2 += 3;
            }

            modifier md2 override {
                s2 += 2;
                _;
                s2 += 5;
            }
        }

        contract base {
            bool private s1;
            int32 internal s2;

            modifier md2 virtual {
                    s2 += 1;
                    _;
                    s2 += 1;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];
    slot[0] = 1;

    assert_eq!(runtime.store.get(&(runtime.vm.address, slot)), None);

    runtime.function("test", Vec::new());

    assert_eq!(
        runtime.store.get(&(runtime.vm.address, slot)).unwrap(),
        &vec!(10, 0, 0, 0)
    );
}

#[test]
fn return_values() {
    // in the modifier syntax, there are no return values
    // however, the generated cfg has the same arguments/returns the function is on attached

    // return simple value
    let mut runtime = build_solidity(
        r##"
        contract c {
            int64 s2;

            function test() md2 public returns (int64) {
                    assert(s2 == 2);
                    s2 += 3;
                    return s2;
            }

            modifier md2 {
                    s2 += 2;
                    _;
                    s2 += 2;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    assert_eq!(runtime.vm.output, Val(5).encode());

    let mut runtime = build_solidity(
        r##"
        struct S {
            int64 f1;
            string f2;
        }

        contract c {
            int64 s2;

            function test() md2 public returns (bool, S) {
                    assert(s2 == 2);
                    s2 += 3;
                    return (true, S({ f1: s2, f2: "Hello, World!" }));
            }

            modifier md2 {
                    s2 += 2;
                    _;
                    s2 += 2;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct StructS(bool, u64, String);

    assert_eq!(
        runtime.vm.output,
        StructS(true, 5, String::from("Hello, World!")).encode()
    );
}

#[test]
fn mutability() {
    let ns = parse_and_resolve(
        r#"
        contract c {
            uint64 var;
            modifier foo(uint64 x) { _; }

            function bar() foo(var) public pure {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function declared ‘pure’ but this expression reads from state"
    );

    let ns = parse_and_resolve(
        r#"
        contract c {
            uint64 var;
            modifier foo() { uint64 x = var; _; }

            function bar() foo() public pure {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function declared ‘pure’ but this expression reads from state"
    );

    let ns = parse_and_resolve(
        r#"
        contract base {
            modifier foo() virtual {
                _;
            }
        }

        contract apex is base {
            function foo() public override {}
        }"#,
        Target::Substrate {
            address_length: 32,
            value_length: 16,
        },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function ‘foo’ overrides modifier"
    );
}

#[test]
fn repeated_modifier() {
    let mut runtime = build_solidity(
        r##"
        contract Test {
            modifier notZero(uint64 num) {
                require(num != 0, "invalid number");
                _;
            }

            function contfunc(uint64 num1, uint64 num2) public notZero(num1) notZero(num2) {
                // any code
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("contfunc", (1u64, 0u64).encode());
    runtime.function_expect_failure("contfunc", (0u64, 0u64).encode());
    runtime.function_expect_failure("contfunc", (0u64, 1u64).encode());
    runtime.function("contfunc", (1u64, 1u64).encode());
}
