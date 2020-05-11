use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, first_warning, no_errors};
use solang::{parse_and_resolve, Target};

#[derive(Debug, PartialEq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn contract_name() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            function test() public {}
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "function cannot have same name as the contract"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            enum test { a}
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            bool test;
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            struct test { bool a; }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "test is already defined as a contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            function f() public {
                int test;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "declaration of `test\' shadows contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            function f(int test) public {
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "declaration of `test\' shadows contract name"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            function f() public returns (int test) {
                return 0;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_warning(errors),
        "declaration of `test\' shadows contract name"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "circular reference creating contract ‘a’"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "circular reference creating contract ‘a’"
    );
}

#[test]
fn contract_type() {
    let (_, errors) = parse_and_resolve(
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
                printer y = x;
            }
        }"#,
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                printer x = address(102);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "implicit conversion from uint8 to address not allowed"
    );

    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "conversion from uint8 to contract printer not possible"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract printer {
            function test() public returns (printer) {
                return new printer();
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "new cannot construct current contract ‘printer’"
    );

    let (_, errors) = parse_and_resolve(
        r#"
        contract printer {
            function test() public returns (printer) {
                return new printer({});
            }
        }"#,
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "new cannot construct current contract ‘printer’"
    );
}

#[test]
fn external_call() {
    let (_, errors) = parse_and_resolve(
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
        Target::Substrate,
    );

    assert_eq!(first_error(errors), "duplicate argument with name ‘t’");

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

    assert_eq!(runtime.vm.scratch, Ret(1020).encode());
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

    runtime.function_expect_return("test", Vec::new(), 1);

    assert_eq!(
        runtime.vm.scratch,
        RevertReturn(0x08c3_79a0, "The reason why".to_string()).encode()
    );
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

    runtime.function_expect_return("test", Vec::new(), 1);

    let expected = RevertReturn(0x08c3_79a0, "Hello, World!".to_string()).encode();

    println!(
        "{} == {}",
        hex::encode(&runtime.vm.scratch),
        hex::encode(&expected)
    );

    assert_eq!(runtime.vm.scratch, expected);
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

    assert_eq!(runtime.vm.scratch, Ret(1020).encode());
}
