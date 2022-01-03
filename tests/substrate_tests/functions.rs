use parity_scale_codec::Encode;
use parity_scale_codec_derive::Decode;

use crate::build_solidity;

#[test]
fn constructors() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 result;

            constructor() public {
                result = 1;
            }

            constructor(uint64 x) public {
                result = x;
            }

            function get() public returns (uint64) {
                return result;
            }
        }",
    );

    runtime.constructor(0, Vec::new());
    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(1).encode());

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 result;

            constructor() public {
                result = 1;
            }

            constructor(uint64 x) public {
                result = x;
            }

            function get() public returns (uint64) {
                return result;
            }
        }",
    );

    runtime.constructor(1, Val(0xaa_bb_cc_dd).encode());
    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(0xaa_bb_cc_dd).encode());
}

#[test]
#[should_panic]
fn constructor_wrong_selector() {
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 result;

            constructor() public {
                result = 1;
            }

            constructor(uint64 x) public {
                result = x;
            }

            function get() public returns (uint64) {
                return result;
            }
        }",
    );

    runtime.raw_constructor(vec![0xaa, 0xbb, 0xcc, 0xdd]);
}

#[test]
fn fallback() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }

            fallback() external {
                result = 356;
            }
        }",
    );

    runtime.raw_function([0xaa, 0xbb, 0xcc, 0xdd, 0xff].to_vec());
    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(356).encode());
}

#[test]
#[should_panic]
fn function_wrong_selector() {
    let mut runtime = build_solidity(
        "
        contract test {
            uint64 result;

            constructor() public {
                result = 1;
            }

            constructor(uint64 x) public {
                result = x;
            }

            function get() public returns (uint64) {
                return result;
            }
        }",
    );

    runtime.raw_function(vec![0xaa, 0xbb, 0xcc, 0xdd]);
}

#[test]
#[should_panic]
fn nofallback() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }
        }",
    );

    runtime.raw_function([0xaa, 0xbb, 0xcc, 0xdd, 0xff].to_vec());
    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(356).encode());
}

#[test]
fn test_overloading() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 result = 1;

            constructor() public {
                foo(true);
                assert(result == 102);
                foo(500);
                assert(result == 510);
            }

            function foo(bool x) private {
                if (x) {
                    result = 102;
                }
            }

            function foo(uint32 x) private {
                result = x + 10;
            }
        }",
    );

    runtime.constructor(0, Vec::new());
}

#[test]
fn shadowing() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let src = "
    contract test {
        uint64 result;

        function goodset(uint64 val) public {
            result = val;
        }

        function badset(uint64 val) public {
            uint64 result = val;
        }

        function get() public returns (uint64) {
            return result;
        }
    }";

    // parse
    let mut runtime = build_solidity(src);

    runtime.constructor(0, Vec::new());

    runtime.function("goodset", Val(0x1234_5678_9abc_def0).encode());

    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(0x1234_5678_9abc_def0).encode());

    runtime.function("badset", Val(1).encode());

    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.output, Val(0x1234_5678_9abc_def0).encode());
}

#[test]
fn test_loops() {
    // parse
    let mut runtime = build_solidity(
        "
        contract test {
            uint32 result = 1;

            constructor() public {
                uint32 n = 0;
                for (uint32 i = 0; i < 1000; i += 100) {
                    n += 1;
                }
                assert(n == 10);

                n = 0;
                for (uint32 i = 0; i < 1000; i += 100) {
                    if (true)
                        continue;
                    n += 1;
                }
                assert(n == 0);

                n = 0;
                for (uint32 i = 0; i < 1000; i += 100) {
                    n += 1;
                    break;
                }
                assert(n == 1);

                n = 0;

                while (n < 10) {
                    n += 9;
                }
                assert(n == 18);

                n = 0;

                while (false) {
                    n += 1000;
                }
                assert(n == 0);

                do {
                    n += 9;
                }
                while(false);

                assert(n == 9);
            }
        }",
    );

    runtime.constructor(0, Vec::new());
}

#[test]
fn test_example() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(i32);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(i64);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct ValBool(bool);

    // parse
    let src = include_str!("../../examples/example.sol");

    let mut runtime = build_solidity(src);

    runtime.constructor(0, Val32(102).encode());

    runtime.function("is_zombie_reaper", Vec::new());

    assert_eq!(runtime.vm.output, ValBool(false).encode());

    runtime.function("reap_processes", Vec::new());

    runtime.function("run_queue", Vec::new());
}

#[test]
fn test_large_vals() {
    // parse
    let src = "
        contract test {
            function large() public returns (int) {
                return 102;
            }

            function large2(int x) public returns (int) {
                return x + 100;
            }

            function doda() public {
                int x = large();
                assert(large2(10) == 110);
            }
        }";

    let mut runtime = build_solidity(src);

    runtime.constructor(0, Vec::new());

    runtime.function("doda", Vec::new());
}

#[test]
fn args_and_returns() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(i32);

    let mut runtime = build_solidity(
        "
        contract foobar {
            function foo1() public returns (int32 a) {
                a = -102;
            }

            function foo2() public returns (int32 a) {
                a = -102;
                return 553;
            }
        }",
    );

    runtime.function("foo1", Vec::new());

    assert_eq!(runtime.vm.output, Val32(-102).encode());

    runtime.function("foo2", Vec::new());

    assert_eq!(runtime.vm.output, Val32(553).encode());
}

#[test]
fn named_argument_call() {
    let mut runtime = build_solidity(
        "
        contract foobar {
            function foo1(bool x) public returns (int32 a) {
                return 2;
            }

            function foo1(uint32 x) public returns (int32 a) {
                a = bar({});
            }

            function bar() private returns (int32) {
                return 1;
            }

            function test() public {
                assert(foo1({ x: true }) == 2);
                assert(foo1({ x: 102 }) == 1);
            }
        }",
    );

    runtime.function("test", Vec::new());
}

#[test]
fn positional_argument_call() {
    let mut runtime = build_solidity(
        "
        contract foobar {
            function foo1(bool x) public returns (int32 a) {
                return 2;
            }

            function foo1(uint32 x) public returns (int32 a) {
                return 1;
            }

            function test() public {
                assert(foo1(true) == 2);
                assert(foo1(102) == 1);
            }
        }",
    );

    runtime.function("test", Vec::new());
}

#[test]
fn print() {
    let mut runtime = build_solidity(
        r#"
    contract foobar {
        function test() public {
            print("Hello, world");
        }
    }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn destructuring_call() {
    let mut runtime = build_solidity(
        r#"
        contract c {
            function func1() public returns (int32, bool) {
                return (102, true);
            }

            function test() public {
                (int32 a, bool b) = func1();

                assert(a == 102 && b == true);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract c {
            function func1(int32 x) public returns (int32, bool) {
                return (102 + x, true);
            }

            function test() public {
                (int32 a, bool b) = func1({x: 5});

                assert(a == 107 && b == true);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        contract c {
            function test() public {
                b x = new b();
                (int32 a, bool b) = x.func1({x: 5});

                assert(a == 107 && b == true);

                (a, b) = x.func1(-1);

                assert(a == 101 && b == true);
            }
        }

        contract b {
            function func1(int32 x) public returns (int32, bool) {
                return (102 + x, true);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}

#[test]
fn global_functions() {
    let mut runtime = build_solidity(
        r#"
        function global_function() pure returns (uint32) {
            return 102;
        }

        contract c {
            function test() public {
                uint64 x = global_function();

                assert(x == 102);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        function global_function() pure returns (uint32) {
            return 102;
        }

        function global_function2() pure returns (uint32) {
            return global_function() + 5;
        }

        contract c {
            function test() public {
                uint64 x = global_function2();

                assert(x == 107);
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r#"
        function global_function() pure returns (uint32) {
            return 102;
        }

        function global_function2() pure returns (uint32) {
            return global_function() + 5;
        }

        contract c {
            function test() public {
                function() internal returns (uint32) ftype = global_function2;

                uint64 x = ftype();

                assert(x == 107);
            }
        }"#,
    );

    runtime.function("test", Vec::new());
}
