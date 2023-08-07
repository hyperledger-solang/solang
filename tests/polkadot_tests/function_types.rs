// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::{Decode, Encode};

#[test]
fn simple_test() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }
        }"##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.output(), 1000u32.encode());
}

#[test]
fn internal_function_type_in_contract_storage() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"##,
    );

    runtime.function("set_op", false.encode());

    runtime.function("test", Args(100, 10).encode());

    assert_eq!(runtime.output(), 110u32.encode());
}

#[test]
#[should_panic]
fn internal_function_not_init_called() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function(int32,int32) internal returns (int32) func;

            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }

            function set_op(bool action) public {
                if (action) {
                    func = mul;
                } else {
                    func = add;
                }
            }

            function test(int32 a, int32 b) public returns (int32) {
                return func(a, b);
            }
        }"##,
    );

    // don't call this runtime.function("set_op", false.encode());

    runtime.function("test", Args(100, 10).encode());
}

#[test]
fn base_contract_function() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft is Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = Arith.mul;
                } else {
                    func = Arith.add;
                }

                return func(a, b);
            }
        }

        abstract contract Arith {
            function mul(int32 a, int32 b) internal returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal returns (int32) {
                return a + b;
            }
        }
        "##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.output(), 1000u32.encode());
}

#[test]
fn virtual_contract_function() {
    #[derive(Debug, PartialEq, Eq, Encode, Decode)]
    struct Args(bool, u32, u32);

    let mut runtime = build_solidity(
        r##"
        contract ft is Arith {
            function mul(int32 a, int32 b) internal override returns (int32) {
                return a * b * 10;
            }

            function add(int32 a, int32 b) internal override returns (int32) {
                return a + b + 10;
            }
        }

        contract Arith {
            function test(bool action, int32 a, int32 b) public returns (int32) {
                function(int32,int32) internal returns (int32) func;

                if (action) {
                    func = mul;
                } else {
                    func = add;
                }

                return func(a, b);
            }

            function mul(int32 a, int32 b) internal virtual returns (int32) {
                return a * b;
            }

            function add(int32 a, int32 b) internal virtual returns (int32) {
                return a + b;
            }
        }
        "##,
    );

    runtime.function("test", Args(true, 100, 10).encode());

    assert_eq!(runtime.output(), 10000u32.encode());
}

// external function types tests
#[test]
fn ext() {
    let mut runtime = build_solidity(
        r#"
        contract ft {
            function test() public {
                function(int32) external returns (bool) func = this.foo;

                assert(address(this) == func.address);
                assert(func.selector == hex"42761137");
            }

            function foo(int32) public returns (bool) {
                return false;
            }
        }"#,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function test() public {
                function(int32) external returns (uint64) func = this.foo;

                assert(func{flags: 8}(102) == 0xabbaabba);
            }

            function foo(int32) public returns (uint64) {
                return 0xabbaabba;
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function test() public {
                function(int32) external returns (uint64) func = this.foo;

                bar(func);
            }

            function foo(int32) public returns (uint64) {
                return 0xabbaabba;
            }

            function bar(function(int32) external returns (uint64) f) internal {
                assert(f{flags: 8}(102) == 0xabbaabba);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function test() public {
                function(int32) external returns (uint64) func = this.foo;

                bar(func);
            }

            function foo(int32) public returns (uint64) {
                return 0xabbaabba;
            }

            function bar(function(int32) external returns (uint64) f) internal {
                assert(f{flags: 8}(102) == 0xabbaabba);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    println!("return external function type from public function");

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function test() public {
                function(int32) external returns (uint64) func = this.foo;

                this.bar{flags: 8}(func);
            }

            function foo(int32) public returns (uint64) {
                return 0xabbaabba;
            }

            function bar(function(int32) external returns (uint64) f) public {
                assert(f{flags: 8}(102) == 0xabbaabba);
            }
        }"##,
    );

    runtime.function("test", Vec::new());

    println!("external function type in storage");

    let mut runtime = build_solidity(
        r##"
        contract ft {
            function(int32) external returns (uint64) func;

            function test1() public {
                func = this.foo;
            }

            function test2() public {
                this.bar{flags: 8}(func);
            }

            function foo(int32) public returns (uint64) {
                return 0xabbaabba;
            }

            function bar(function(int32) external returns (uint64) f) public {
                assert(f{flags: 8}(102) == 0xabbaabba);
            }
        }"##,
    );

    runtime.function("test1", Vec::new());
    runtime.function("test2", Vec::new());
}

/// Test external function decoding
#[test]
fn encode_decode_ext_func() {
    let mut runtime = build_solidity(
        r#"
        contract A {
            @selector([0,0,0,0])
            function a() public pure returns (uint8) {
                return 127;
            }

            function it() public view returns (address) {
                return address(this);
            }
        }
        
        contract B {
            function encode_decode_call(address a) public returns (uint8) {
                bytes4 selector = hex"00000000";
    	        bytes enc = abi.encode(a, selector);
                function() external returns (uint8) dec2 = abi.decode(enc, (function() external returns (uint8)));
                return dec2{flags: 8}();
            }

            function decode_call(function() external returns(uint8) func) public returns (uint8) {
                bytes enc = abi.encode(func);
                print("{}  ".format(enc));
                function() external returns (uint8) dec2 = abi.decode(enc, (function() external returns (uint8)));
                return dec2{flags: 8}();
            }
        }
        "#,
    );

    runtime.function("it", vec![]);
    let mut address_of_a = runtime.output();

    runtime.set_account(1);
    runtime.function("encode_decode_call", address_of_a.clone());
    assert_eq!(runtime.output(), 127u8.encode());

    address_of_a.extend([0, 0, 0, 0].iter());
    runtime.function("decode_call", address_of_a);
    assert_eq!(runtime.output(), 127u8.encode());
}
