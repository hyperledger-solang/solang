
use parity_scale_codec::Encode;
use parity_scale_codec_derive::{Encode, Decode};

use super::{build_solidity, first_error};
use solang::{parse_and_resolve, Target};

#[test]
fn constructors() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let (runtime, mut store) = build_solidity("
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
        }");

    runtime.constructor(&mut store, 0, Vec::new());
    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, Val(1).encode());

        // parse
        let (runtime, mut store) = build_solidity("
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
        }");

    runtime.constructor(&mut store, 1, Val(0xaabbccdd).encode());
    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, Val(0xaabbccdd).encode());
}

#[test]
fn fallback() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }

            function() external {
                result = 356;
            }
        }");

    runtime.raw_function(&mut store, [ 0xaa, 0xbb, 0xcc, 0xdd, 0xff ].to_vec());
    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, Val(356).encode());
}

#[test]
#[should_panic]
fn nofallback() {
    #[derive(Debug, PartialEq, Encode, Decode)]
    struct Val(u64);

    // parse
    let (runtime, mut store) = build_solidity("
        contract test {
            int64 result = 102;

            function get() public returns (int64) {
                return result;
            }
        }");

    runtime.raw_function(&mut store, [ 0xaa, 0xbb, 0xcc, 0xdd, 0xff ].to_vec());
    runtime.function(&mut store, "get", Vec::new());

    assert_eq!(store.scratch, Val(356).encode());
}

#[test]
fn test_overloading() {
    // parse
    let (runtime, mut store) = build_solidity("
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
        }");

    runtime.constructor(&mut store, 0, Vec::new());
}

#[test]
fn mutability() {
    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 1844674;

            function bar() public pure returns (int64) {
                return foo;
            }
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "function declared pure but reads contract storage");

    let (_, errors) = parse_and_resolve(
        "contract test {
            int64 foo = 1844674;

            function bar() public view {
                foo = 102;
            }
        }", &Target::Substrate);

    assert_eq!(first_error(errors), "function declared view but writes contract storage");
}