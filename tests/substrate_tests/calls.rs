use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use crate::{build_solidity, first_error, no_errors, parse_and_resolve};
use solang::Target;

#[derive(Debug, PartialEq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn revert() {
    let mut runtime = build_solidity(
        r##"
        contract bar {
            function test() public {
                revert("yo!");
            }

            function a() public {
                b();
            }

            function b() public {
                c();
            }

            function c() public {
                d();
            }

            function d() public {
                revert("revert value has to be passed down the stack");
            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);

    assert_eq!(
        runtime.vm.output,
        RevertReturn(0x08c3_79a0, "yo!".to_string()).encode()
    );

    runtime.function_expect_return("a", Vec::new(), 1);

    assert_eq!(
        runtime.vm.output,
        RevertReturn(
            0x08c3_79a0,
            "revert value has to be passed down the stack".to_string()
        )
        .encode()
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                revert();
            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);

    assert_eq!(runtime.vm.output.len(), 0);
}

#[test]
fn require() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test1() public {
                require(false, "Program testing can be used to show the presence of bugs, but never to show their absence!");
            }

            function test2() public {
                require(true, "Program testing can be used to show the presence of bugs, but never to show their absence!");
            }
        }"##,
    );

    runtime.function_expect_return("test1", Vec::new(), 1);

    assert_eq!(
        runtime.vm.output,
        RevertReturn(0x08c3_79a0, "Program testing can be used to show the presence of bugs, but never to show their absence!".to_string()).encode()
    );

    runtime.function("test2", Vec::new());

    assert_eq!(runtime.vm.output.len(), 0);
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
        Target::Substrate,
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
        Target::Substrate,
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "implicit conversion from uint8 to address not allowed"
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "conversion from uint8 to contract printer not possible"
    );

    let ns = parse_and_resolve(
        r#"
        contract printer {
            function test() public returns (printer) {
                return new printer();
            }
        }"#,
        Target::Substrate,
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
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "new cannot construct current contract ‘printer’"
    );
}

#[test]
fn input_wrong_size() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test(int32 x) public {
            }
        }"##,
    );

    runtime.function_expect_return("test", b"A".to_vec(), 1);

    // the decoder does check if there is too much data
    runtime.function_expect_return("test", b"ABCDE".to_vec(), 1);
}

#[test]
fn external_call_not_exist() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = other(address(102));

                o.test();
            }
        }

        contract other {
            function test() public {

            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);
}

#[test]
fn contract_already_exists() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other{salt: 0}();

                other t = new other{salt: 0}();
            }
        }

        contract other {
            function test() public {

            }
        }"##,
    );

    runtime.function_expect_return("test", Vec::new(), 1);

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other();

                other t = new other();
            }
        }

        contract other {
            function test() public {

            }
        }"##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn try_catch_external_calls() {
    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "try returns list has 1 entries while function returns 2 values"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32, int[2] storage) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "type ‘int256[2] storage’ does not match return value of function ‘bool’"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 y, bool) {
                    x = y;
                } catch (bytes) {
                    x = 2;
                }
                assert(x == 102);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 y, bool) {
                    x = y;
                } catch (bytes c) {
                    assert(c == hex"a079c3080c666f6f");
                    x = 2;
                }
                assert(x == 2);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                revert("foo");
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32, bool) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "catch can only take ‘bytes memory’, not ‘string’"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 x, bool) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "x is already declared");

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 bla, bool) {
                    x = bla;
                } catch Foo(bytes memory f) {
                    x = 105;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "only catch ‘Error’ is supported, not ‘Foo’"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 bla, bool) {
                    x = bla;
                } catch Error(bytes memory f) {
                    x = 105;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "catch Error(...) can only take ‘string memory’, not ‘bytes’"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                other o = new other();

                try o.test(1) {
                    print("shouldn't be here");
                    assert(false);
                } catch Error(string foo) {
                    print(foo);
                    assert(foo == "yes");
                } catch (bytes c) {
                    print("shouldn't be here");
                    assert(false);
                }

                try o.test(2) {
                    print("shouldn't be here");
                    assert(false);
                } catch Error(string foo) {
                    print(foo);
                    assert(foo == "no");
                } catch (bytes c) {
                    print("shouldn't be here");
                    assert(false);
                }

                try o.test(3) {
                    print("shouldn't be here");
                    assert(false);
                } catch Error(string foo) {
                    print("shouldn't be here");
                    assert(false);
                } catch (bytes c) {
                    assert(c.length == 0);
                }
            }
        }

        contract other {
            function test(int x) public {
                if (x == 1) {
                    revert("yes");
                } else if (x == 2) {
                    revert("no");
                } else {
                    revert();
                }
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract dominator {
            child c;

            function create_child() public {
                c = new child();
            }

            function call_child() public view returns (int64) {
                return c.get_a();
            }

            function test() public pure returns (int32) {
                try c.go_bang() returns (int32 l) {
                    print("try call success");
                    return 8000;
                }
                catch Error(string l) {
                    print("try error path");
                    print(l);
                    return 4000;
                }
                catch (bytes) {
                    print("try catch path");
                    return 2000;
                }

            }
        }

        contract child {
            int64 a;
            constructor() public {
                a = 102;
            }

            function get_a() public view returns (int64) {
                return a;
            }

            function set_a(int64 l) public {
                a = l;
            }

            function go_bang() public pure returns (int32) {
                revert("gone bang in child");
            }
        }"##,
    );

    runtime.function("create_child", Vec::new());

    runtime.function("test", Vec::new());

    assert_eq!(runtime.vm.output, Ret(4000).encode());
}

#[test]
fn try_catch_constructor() {
    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                try new other() returns (int32) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "type ‘int32’ does not match return value of function ‘contract other’"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                try new other() returns (int32, int[2] storage) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "constructor returns single contract, not 2 values"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                int x;
                try new other() {
                    x = 102;
                } catch (bytes) {
                    x = 2;
                }
                assert(x == 102);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                int x;
                try new other({foo: true}) returns (other o) {
                    (x, bool yata) = o.test();
                } catch (bytes) {
                    x = 2;
                }
                assert(x == 102);
            }
        }

        contract other {
            constructor(bool foo) public {
                //
            }

            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                int32 x = 0;
                try new other(true) {
                    x = 1;
                } catch (bytes c) {
                    assert(c == hex"a079c3080c666f6f");
                    x = 2;
                }
                assert(x == 2);
            }
        }

        contract other {
            constructor(bool foo) public {
                revert("foo");
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                try new int32[](2) returns (int32, bool) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "try only supports external calls or constructor calls"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function f() public {
                x : 1
            }
        }"##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "expected code block, not list of named arguments"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                try new other()
                catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public  {
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "code block missing for no catch"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test() public {
                try new other() {
                    x = 1;
                } {
                    x= 5;
                }
                catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public  {
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "unexpected code block");

    let ns = parse_and_resolve(
        r##"
        contract c {
            function test(other o) public {
                try o.test() {
                    x = 1;
                } {
                    x= 5;
                }
                catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public  {
            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(first_error(ns.diagnostics), "unexpected code block");
}

#[test]
fn local_destructure_call() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                (, bytes32 b, string s) = foo();

                assert(b == "0123");
                assert(s == "abcd");
            }

            function foo() public returns (bool, bytes32, string) {
                return (true, "0123", "abcd");
            }
        }
        "##,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn payable_constructors() {
    // no contructors means constructor is not payable
    // however there is no check for value transfers on constructor so endowment can be received
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test(string a) public {
            }
        }"##,
    );

    runtime.vm.value = 1;
    runtime.constructor(0, Vec::new());

    // contructors w/o payable means can't send value
    // however there is no check for value transfers on constructor so endowment can be received
    let mut runtime = build_solidity(
        r##"
        contract c {
            constructor() public {
                int32 a = 0;
            }

            function test(string a) public {
            }
        }"##,
    );

    runtime.vm.value = 1;
    runtime.constructor(0, Vec::new());

    // contructors w/ payable means can send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            constructor() public payable {
                int32 a = 0;
            }

            function test(string a) public {
            }
        }"##,
    );

    runtime.vm.value = 1;
    runtime.constructor(0, Vec::new());
}

#[test]
fn payable_functions() {
    // function w/o payable means can't send value
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.vm.value = 1;
    runtime.function_expect_return("test", Vec::new(), 1);

    // test both
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() payable public {
            }
            function test2() public {
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.vm.value = 1;
    runtime.function_expect_return("test2", Vec::new(), 1);
    runtime.vm.value = 1;
    runtime.function("test", Vec::new());

    // test fallback and receive
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract c {
            int32 x;

            function get_x() public returns (int32) {
                return x;
            }

            function test() payable public {
                x = 1;
            }

            fallback() external {
                x = 2;
            }

            receive() payable external {
                x = 3;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.vm.value = 1;
    runtime.raw_function(b"abde".to_vec());
    runtime.vm.value = 0;
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Ret(3).encode());

    runtime.vm.value = 0;
    runtime.raw_function(b"abde".to_vec());
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Ret(2).encode());

    let mut runtime = build_solidity(
        r##"
        contract c {
            int32 x;

            function get_x() public returns (int32) {
                return x;
            }

            function test() payable public {
                x = 1;
            }

            receive() payable external {
                x = 3;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.vm.value = 1;
    runtime.raw_function(b"abde".to_vec());
    runtime.vm.value = 0;
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Ret(3).encode());

    runtime.vm.value = 0;
    runtime.raw_function_return(1, b"abde".to_vec());
    let mut runtime = build_solidity(
        r##"
        contract c {
            int32 x;

            function get_x() public returns (int32) {
                return x;
            }

            function test() payable public {
                x = 1;
            }

            fallback() external {
                x = 2;
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.vm.value = 1;
    runtime.raw_function_return(1, b"abde".to_vec());

    runtime.vm.value = 0;
    runtime.raw_function(b"abde".to_vec());
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Ret(2).encode());

    let ns = parse_and_resolve(
        r##"
        contract c {
            receive() public {

            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "receive function must be declared external"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            receive() external  {

            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "receive function must be declared payable"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            fallback() payable external {

            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "fallback function must not be declare payable, use ‘receive() external payable’ instead"
    );

    let ns = parse_and_resolve(
        r##"
        contract c {
            fallback() public {

            }
        }
        "##,
        Target::Substrate,
    );

    assert_eq!(
        first_error(ns.diagnostics),
        "fallback function must be declared external"
    );
}

#[test]
fn hash_tests() {
    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes32 hash = keccak256("Hello, World!");

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes memory s = "Hello, World!";
                bytes32 hash = keccak256(s);

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            bytes s = "Hello, World!";

            function test() public {
                bytes32 hash = keccak256(s);

                assert(hash == hex"acaf3289d7b601cbd114fb36c4d29c85bbfd5e133f14cb355c3fd8d99367964f");
            }
        }"##,
    );

    runtime.constructor(0, Vec::new());
    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes32 hash = sha256("Hello, World!");

                assert(hash == hex"dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes32 hash = blake2_256("Hello, World!");

                assert(hash == hex"511bc81dde11180838c562c82bb35f3223f46061ebde4a955c27b3f489cf1e03");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes16 hash = blake2_128("Hello, World!");

                assert(hash == hex"3895c59e4aeb0903396b5be3fbec69fe");
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract tester {
            function test() public {
                bytes20 hash = ripemd160("Hello, World!");

                assert(hash == hex"527a6a4b9a6da75607546842e0e00105350b1aaf");
            }
        }"##,
    );

    runtime.function("test", Vec::new());
}
