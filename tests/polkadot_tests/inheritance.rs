// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};
use solang::codegen::{OptimizationLevel, Options};
use solang::file_resolver::FileResolver;
use solang::Target;
use std::ffi::OsStr;

#[test]
fn test_abstract() {
    let mut cache = FileResolver::default();

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
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_polkadot(),
        &Options {
            opt_level: OptimizationLevel::Default,
            log_runtime_errors: false,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            ..Default::default()
        },
        vec!["unknown".to_string()],
        "0.0.1",
    );

    assert!(!ns.diagnostics.any_errors());

    assert_eq!(contracts.len(), 1);

    let mut cache = FileResolver::default();

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
        OsStr::new("a.sol"),
        &mut cache,
        Target::default_polkadot(),
        &Options {
            opt_level: OptimizationLevel::Default,
            log_runtime_errors: false,
            log_prints: true,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            ..Default::default()
        },
        vec!["unknown".to_string()],
        "0.0.1",
    );

    assert!(!ns.diagnostics.any_errors());

    assert_eq!(contracts.len(), 1);
}
#[test]
fn inherit_variables() {
    let mut runtime = build_solidity(
        r##"
        contract b is a {
            uint16 public foo = 65535;
        }

        abstract contract a {
            uint16 private foo = 102;
        }"##,
    );

    runtime.constructor(0, Vec::new());

    let mut slot = [0u8; 32];

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(102, 0));

    slot[0] = 1;

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(0xff, 0xff));

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

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(102, 0));

    slot[0] = 1;

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(0xff, 0xff));
}

#[test]
fn call_inherited_function() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
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

    assert_eq!(runtime.output(), Val(105).encode());

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

    assert_eq!(runtime.output(), Val(36).encode());

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

    assert_eq!(runtime.output(), Val(161720).encode());

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
    runtime.set_transferred_value(0);
    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());
    assert_eq!(runtime.output(), Val(1).encode());

    runtime.raw_function([0x45, 0x55, 0x75, 0x78, 1].to_vec());
    assert_eq!(runtime.output(), Val(2).encode());

    runtime.raw_function([0x36, 0x8E, 0x4A, 0x7F, 1, 2, 3, 4, 5, 6, 7, 8].to_vec());
    assert_eq!(runtime.output(), Val(3).encode());
}

#[test]
fn test_override() {
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
    assert_eq!(runtime.storage()[&slot], vec!(3));

    runtime.set_transferred_value(1);
    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());

    let slot = [0u8; 32];

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(2));

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
    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(3));

    runtime.set_transferred_value(0);
    runtime.raw_function([0xC2, 0x98, 0x55, 0x78].to_vec());

    let slot = [0u8; 32];

    assert_eq!(runtime.contracts()[0].storage[&slot], vec!(2));
}

#[test]
fn base_contract() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(u32);

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

    assert_eq!(runtime.output(), Val(102).encode());
}

#[test]
fn base_contract_on_constructor() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val64(u64);

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

    assert_eq!(runtime.output(), Val(102).encode());

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

    assert_eq!(runtime.output(), Val(104).encode());

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

    assert_eq!(runtime.output(), Val64(12).encode());

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

    assert_eq!(runtime.output(), Val64(12).encode());
}

#[test]
fn call_base_function_via_basename() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val(i32);
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Val64(u64);

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            function bar() public returns (uint64) {
                return a.foo();
            }
        }

        abstract contract b is a {
            function foo() internal override returns (uint64) {
                return 2;
            }
        }

        abstract contract a {
            function foo() internal virtual returns (uint64) {
                return 1;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), Val64(1).encode());

    let mut runtime = build_solidity(
        r##"
        contract c is b {
            uint64 constant private C = 100;
            function bar() public returns (uint64) {
                return a.foo({ x: C });
            }
        }

        abstract contract b is a {
            uint64 constant private C = 300;
            function foo(uint64 x) internal override returns (uint64) {
                return 2;
            }
        }

        abstract contract a {
            uint64 constant private C = 200;
            function foo(uint64 x) internal virtual returns (uint64) {
                return 1 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), Val64(101).encode());
}

#[test]
fn simple_interface() {
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

    assert_eq!(runtime.output(), 200u32.encode());
}

#[test]
fn test_super() {
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

        abstract contract a {
            uint64 var;

            function foo() internal virtual {
                var = 102;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), 102u64.encode());

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

        abstract contract a {
            uint64 var;

            function foo(uint64 x) internal virtual {
                var = 102 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), 112u64.encode());

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

    assert_eq!(runtime.output(), 112u64.encode());

    // super should not consider interfaces
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

        interface a {
            function foo(uint64 x) external returns (uint64);
        }

        contract aa {
            function foo(uint64 x) public virtual returns (uint64) {
                return 202 + x;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("bar", Vec::new());

    assert_eq!(runtime.output(), 212u64.encode());
}

#[test]
fn var_or_function() {
    let mut runtime = build_solidity(
        r##"
        contract x is c {
            function f1() public returns (int64) {
                return selector;
            }

            function f2() public returns (int64)  {
                function() external returns (int64) a = this.selector;
                return a{flags: 8}();
            }
        }

        contract c {
            int64 public selector = 102;
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("f1", Vec::new());

    assert_eq!(runtime.output(), 102u64.encode());

    runtime.function("f2", Vec::new());

    assert_eq!(runtime.output(), 102u64.encode());
}
