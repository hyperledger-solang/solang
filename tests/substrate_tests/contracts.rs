use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use crate::{build_solidity, first_error, first_warning, no_errors, parse_and_resolve};
use solang::Target;

#[derive(Debug, PartialEq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn contract_name() {
    let ns = parse_and_resolve(
        "contract test {
            function test() public {}
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function cannot have same name as the contract"
    );

    let ns = parse_and_resolve(
        "contract test {
            enum test { a}
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "test is already defined as a contract name"
    );

    let ns = parse_and_resolve(
        "contract test {
            bool test;
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "test is already defined as a contract name"
    );

    let ns = parse_and_resolve(
        "contract test {
            struct test { bool a; }
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "test is already defined as a contract name"
    );

    let ns = parse_and_resolve(
        "contract test {
            function f() public {
                int test;
            }
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "declaration of ‘test’ shadows contract name"
    );

    let ns = parse_and_resolve(
        "contract test {
            function f(int test) public {
            }
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "declaration of ‘test’ shadows contract name"
    );

    let ns = parse_and_resolve(
        "contract test {
            function f() public returns (int test) {
                return 0;
            }
        }",
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_warning(ns.diagnostics),
        "declaration of ‘test’ shadows contract name"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                a y = new a();
            }
        }
        "#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "circular reference creating contract ‘a’"
    );

    let ns = parse_and_resolve(
        r#"
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                c y = new c();
            }
        }

        contract c {
            function x() public {
                a y = new a();
            }
        }
        "#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "circular reference creating contract ‘a’"
    );
}

#[test]
fn contract_type() {
    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                print("In f.test()");
            }
        }

        contract foo {
            function test1(printer x) public {
                address y = x;
            }

            function test2(address x) public {
                printer y = printer(x);
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion to address from contract printer not allowed"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                printer x = printer(address(102));
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    no_errors(ns.diagnostics);

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                print("In f.test()");
            }
        }

        contract foo {
            function test1(printer x) public {
                address y = 102;
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expected ‘address’, found integer"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                print("In f.test()");
            }
        }

        contract foo {
            function test1() public {
                printer y = 102;
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expected ‘contract printer’, found integer"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public returns (printer) {
                return new printer();
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new cannot construct current contract ‘printer’"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public returns (printer) {
                return new printer({});
            }
        }"#,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new cannot construct current contract ‘printer’"
    );
}

#[test]
fn external_call() {
    let ns = parse_and_resolve(
        r##"
        contract c {
            b x;
            function test() public returns (int32) {
                return x.get_x();
            }
        }

        contract b {
            function get_x(int32 t) public returns (int32) {
                return 1;
            }
        }"##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function expects 1 arguments, 0 provided"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            b x;
            function test() public returns (int32) {
                return x.get_x({b: false});
            }
        }

        contract b {
            function get_x(int32 t, bool b) public returns (int32) {
                return 1;
            }
        }"##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "function expects 2 arguments, 1 provided"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10, t: false });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "duplicate argument with name ‘t’"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b({ a: 1, a: 2 });
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(first_error(ns.diagnostics), "duplicate argument name ‘a’");

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Ret(1020).encode());
}

#[test]
fn revert_external_call() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }
            function test() public returns (int32) {
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                x = a;
            }
            function get_x(int32 t) public returns (int32) {
                revert("The reason why");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("test", Vec::new());
}

#[test]
fn revert_constructor() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            b x;
            constructor() public {
            }
            function test() public returns (int32) {
                x = new b(102);
                return x.get_x({ t: 10 });
            }
        }

        contract b {
            int32 x;
            constructor(int32 a) public {
                require(a == 0, "Hello,\
 World!");
            }

            function get_x(int32 t) public returns (int32) {
                return x * t;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function_expect_failure("test", Vec::new());

    assert_eq!(runtime.vm.output.len(), 0);
}

#[test]
fn external_datatypes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u64);

    let mut runtime = build_solidity(
        r##"
        contract c {
            b x;
            constructor() public {
                x = new b(102);
            }

            function test() public returns (int64) {
                strukt k = x.get_x(10, "foobar", true, strukt({ f1: "abcd", f2: address(555555), f3: -1 }));

                assert(k.f1 == "1234");
                assert(k.f2 == address(102));
                return int64(k.f3);
            }
        }

        contract b {
            int x;
            constructor(int a) public {
                x = a;
            }

            function get_x(int t, string s, bool y, strukt k) public returns (strukt) {
                assert(y == true);
                assert(t == 10);
                assert(s == "foobar");
                assert(k.f1 == "abcd");

                return strukt({ f1: "1234", f2: address(102), f3: x * t });
            }
        }

        struct strukt {
            bytes4 f1;
            address f2;
            int f3;
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Ret(1020).encode());
}

#[test]
fn creation_code() {
    let ns = parse_and_resolve(
        r##"
        contract a {
            function test() public {
                    bytes code = type(b).creationCode;
            }
        }

        contract b {
                int x;

                function test() public {
                        a f = new a();
                }
        }
        "##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "circular reference creating contract ‘a’"
    );

    let ns = parse_and_resolve(
        r##"
        contract a {
            function test() public {
                    bytes code = type(a).runtimeCode;
            }
        }"##,
        Target::Substrate { address_length: 32 },
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "containing our own contract code for ‘a’ would generate infinite size contract"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public returns (bytes) {
                bytes runtime = type(b).runtimeCode;

                assert(runtime[0] == 0);
                assert(runtime[1] == 0x61); // a
                assert(runtime[2] == 0x73); // s
                assert(runtime[3] == 0x6d); // m

                bytes creation = type(b).creationCode;

                // on Substrate, they are the same
                assert(creation == runtime);

                return creation;
            }
        }

        contract b {
            int x;
            constructor(int a) public {
                x = a;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(Vec<u8>);

    // return value should be the code for the second contract
    assert_eq!(
        runtime.vm.output,
        Ret(runtime.contracts[1].0.clone()).encode()
    );
}
