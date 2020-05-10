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
fn try_catch() {
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
}
