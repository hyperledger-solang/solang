// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, build_solidity_with_options};
use parity_scale_codec::{Decode, Encode};

#[derive(Debug, PartialEq, Eq, Encode, Decode)]
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

    runtime.function_expect_failure("test", Vec::new());

    assert_eq!(runtime.vm.output.len(), 0);

    runtime.function_expect_failure("a", Vec::new());

    assert_eq!(runtime.vm.output.len(), 0);

    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                revert();
            }
        }"##,
    );

    runtime.function_expect_failure("test", Vec::new());

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

    runtime.function_expect_failure("test1", Vec::new());

    // The reason is lost
    assert_eq!(runtime.vm.output.len(), 0);

    runtime.function("test2", Vec::new());

    assert_eq!(runtime.vm.output.len(), 0);
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

    runtime.function_expect_failure("test", b"A".to_vec());

    // the decoder does check if there is too much data
    runtime.function_expect_failure("test", b"ABCDE".to_vec());
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

    runtime.function_expect_failure("test", Vec::new());
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

    runtime.function_expect_failure("test", Vec::new());

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

    runtime.function_expect_failure("test", Vec::new());

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

    runtime.function_expect_failure("test", Vec::new());

    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Ret(u32);

    let mut runtime = build_solidity(
        r##"
        contract dominator {
            child c;

            function create_child() public {
                c = (new child)();
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
                catch {
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

    runtime.function_expect_failure("test", Vec::new());
}

#[test]
fn try_catch_constructor() {
    let mut runtime = build_solidity(
        r##"
        contract c {
            function test() public {
                int x;
                try (new other)() {
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

            function _ext() public {}
        }
        "##,
    );

    runtime.function_expect_failure("test", Vec::new());
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
    runtime.function_expect_failure("test", Vec::new());

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
    runtime.function_expect_failure("test2", Vec::new());
    runtime.vm.value = 1;
    runtime.function("test", Vec::new());

    // test fallback and receive
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
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
    runtime.raw_function_failure(b"abde".to_vec());
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
    runtime.raw_function_failure(b"abde".to_vec());

    runtime.vm.value = 0;
    runtime.raw_function(b"abde".to_vec());
    runtime.function("get_x", Vec::new());

    assert_eq!(runtime.vm.output, Ret(2).encode());
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

#[test]
fn try_catch_reachable() {
    // ensure that implicit return gets added correctly if catch reachable not
    let _ = build_solidity(
        r##"
        contract AddNumbers { function add(uint256 a, uint256 b) external returns (uint256 c) {c = a + b;} }

        contract Example {
            AddNumbers addContract;
            event StringFailure(string stringFailure);
            event BytesFailure(bytes bytesFailure);

            function exampleFunction(uint256 _a, uint256 _b) public returns (uint256 _c) {

                try addContract.add(_a, _b) returns (uint256 _value) {
                    return (_value);
                } catch Error(string memory _err) {
                    // This may occur if there is an overflow with the two numbers and the `AddNumbers` contract explicitly fails with a `revert()`
                    emit StringFailure(_err);
                } catch (bytes memory _err) {
                    emit BytesFailure(_err);
                }
                _c = 1;
            }
        }"##,
    );

    let _ = build_solidity(
        r##"
        contract AddNumbers { function add(uint256 a, uint256 b) external returns (uint256 c) {c = a + b;} }

        contract Example {
            AddNumbers addContract;
            event StringFailure(string stringFailure);
            event BytesFailure(bytes bytesFailure);

            function exampleFunction(uint256 _a, uint256 _b) public returns (uint256 _c) {

                try addContract.add(_a, _b) returns (uint256 _value) {
                    return (_value);
                } catch Error(string memory _err) {
                    // This may occur if there is an overflow with the two numbers and the `AddNumbers` contract explicitly fails with a `revert()`
                    emit StringFailure(_err);
                } catch (bytes memory _err) {
                    emit BytesFailure(_err);
                    return;
                }
                _c = 1;
            }
        }"##,
    );
}

#[test]
fn log_api_call_return_values_works() {
    let mut runtime = build_solidity_with_options(
        r##"
        contract Test {
            constructor () payable {}
        
            function test() public {
                try new Other() returns (Other o) {
                    o.foo();
                } 
                catch {}
            }
        }
        contract Other {
            function foo() public pure {
                print("hi!");
            }
        }
        "##,
        false,
        true,
    );

    runtime.function("test", vec![]);
    assert_eq!(
        &runtime.printbuf,
        "seal_instantiate=0hi!seal_debug_message=0seal_call=0"
    );
}
