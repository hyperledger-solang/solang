use parity_scale_codec::Encode;
use parity_scale_codec_derive::Decode;

use crate::build_solidity;

#[derive(Debug, PartialEq, Encode, Decode)]
struct RevertReturn(u32, String);

#[test]
fn external_call() {
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
