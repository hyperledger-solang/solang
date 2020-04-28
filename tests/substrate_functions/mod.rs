use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Decode, Encode};

use super::{build_solidity, first_error, first_warning};
use solang::{parse_and_resolve, Target};

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

    assert_eq!(runtime.vm.scratch, Val(1).encode());

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

    assert_eq!(runtime.vm.scratch, Val(0xaa_bb_cc_dd).encode());
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

            function() external {
                result = 356;
            }
        }",
    );

    runtime.raw_function([0xaa, 0xbb, 0xcc, 0xdd, 0xff].to_vec());
    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.scratch, Val(356).encode());
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

    assert_eq!(runtime.vm.scratch, Val(356).encode());
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
fn mutability() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 1844674;

            function bar() public pure returns (int64) {
                return foo;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "function declared pure but reads contract storage"
    );

    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 1844674;

            function bar() public view {
                foo = 102;
            }
        }",
        Target::Substrate,
    );

    assert_eq!(
        first_error(errors),
        "function declared view but writes contract storage"
    );
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

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_warning(errors),
        "declaration of `result\' shadows state variable"
    );

    // parse
    let mut runtime = build_solidity(src);

    runtime.constructor(0, Vec::new());

    runtime.function("goodset", Val(0x1234_5678_9abc_def0).encode());

    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.scratch, Val(0x1234_5678_9abc_def0).encode());

    runtime.function("badset", Val(1).encode());

    runtime.function("get", Vec::new());

    assert_eq!(runtime.vm.scratch, Val(0x1234_5678_9abc_def0).encode());
}

#[test]
fn scopes() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    let src = "
    contract test {
        function goodset() public returns (bool) {
            {
                bool a = true;
            }
            return a;
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "`a\' is not declared");

    let src = "
    contract test {
        function goodset() public returns (uint) {
            for (uint i = 0; i < 10 ; i++) {
                bool a = true;
            }
            return i;
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "`i\' is not declared");
}

#[test]
fn for_forever() {
    let src = "
    contract test {
        function goodset() public returns (bool) {
            for (;;) {
                // ...
            }
            return;
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "unreachable statement");
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
fn test_full_example() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(i32);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val64(i64);

    #[derive(Debug, PartialEq, Encode, Decode)]
    struct ValBool(bool);

    // parse
    let src = include_str!("../../examples/full_example.sol");

    let mut runtime = build_solidity(&src);

    runtime.constructor(0, Val32(102).encode());

    runtime.function("is_zombie_reaper", Vec::new());

    assert_eq!(runtime.vm.scratch, ValBool(false).encode());

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

    let mut runtime = build_solidity(&src);

    runtime.constructor(0, Vec::new());

    runtime.function("doda", Vec::new());
}

#[test]
fn args_and_returns() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val32(i32);

    let src = "
    contract args {
        function foo(bool arg1, uint arg1) public {
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "arg1 is already declared");

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public returns (address arg2, uint) {
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "arg2 is already declared");

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public returns (address, uint) {
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "missing return statement");

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

    assert_eq!(runtime.vm.scratch, Val32(-102).encode());

    runtime.function("foo2", Vec::new());

    assert_eq!(runtime.vm.scratch, Val32(553).encode());
}

#[test]
fn named_argument_call() {
    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false });
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_error(errors),
        "function expects 2 arguments, 1 provided"
    );

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo[1]({ arg1: false });
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "unexpected array type");

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg3: 1 });
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_error(errors),
        "missing argument ‘arg2’ to function ‘foo’"
    );

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo({ arg1: false, arg2: true });
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_error(errors),
        "conversion from bool to uint256 not possible"
    );

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
    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo(false);
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_error(errors),
        "function expects 2 arguments, 1 provided"
    );

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo[1](false, 1);
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(first_error(errors), "unexpected array type");

    let src = "
    contract args {
        function foo(bool arg1, uint arg2) public {
        }

        function bar() private {
            foo(1, false);
        }
    }";

    let (_, errors) = parse_and_resolve(&src, Target::Substrate);

    assert_eq!(
        first_error(errors),
        "conversion from uint8 to bool not possible"
    );

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
