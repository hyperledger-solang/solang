use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, no_errors};
use solang::{parse_and_resolve, Target};

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
        runtime.vm.scratch,
        RevertReturn(0x08c3_79a0, "yo!".to_string()).encode()
    );

    runtime.function_expect_return("a", Vec::new(), 1);

    assert_eq!(
        runtime.vm.scratch,
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

    assert_eq!(runtime.vm.scratch.len(), 0);
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
        runtime.vm.scratch,
        RevertReturn(0x08c3_79a0, "Program testing can be used to show the presence of bugs, but never to show their absence!".to_string()).encode()
    );

    runtime.function("test2", Vec::new());

    assert_eq!(runtime.vm.scratch.len(), 0);
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
                printer y = printer(x);
            }
        }"#,
        Target::Substrate,
    );

    no_errors(errors);

    let (_, errors) = parse_and_resolve(
        r#"
        contract printer {
            function test() public {
                printer x = printer(address(102));
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
fn input_wrong_size() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test(int32 x) public {
            }
        }"##,
    );

    runtime.function_expect_return("test", b"A".to_vec(), 3);

    runtime.function_expect_return("test", b"ABCDE".to_vec(), 3);
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

    runtime.function_expect_return("test", Vec::new(), 4);
}

#[test]
fn contract_already_exists() {
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

    runtime.function_expect_return("test", Vec::new(), 4);
}

#[test]
fn try_catch_external_calls() {
    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "try returns list has 2 entries while function returns 1 values"
    );

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
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

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "catch can only take ‘bytes memory’, not ‘string’"
    );

    let (_, errors) = parse_and_resolve(
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

    assert_eq!(first_error(errors), "x is already declared");

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "only catch ‘Error’ is supported, not ‘Foo’"
    );

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
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
        
            function call_child() public pure returns (int64) {
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

    assert_eq!(runtime.vm.scratch, Ret(4000).encode());
}

#[test]
fn try_catch_constructor() {
    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "type ‘int32’ does not match return value of function ‘contract other’"
    );

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "constructor returns single contract, not 2 values"
    );

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                int x;
                try new other()  {
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

    let (_, errors) = parse_and_resolve(
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
        first_error(errors),
        "try only supports external calls or constructor calls"
    );
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
