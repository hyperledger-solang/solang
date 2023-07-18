// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity;
use parity_scale_codec::Encode;

#[test]
fn constructor_buf_too_small() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {

        function write_bytes_failure(uint8 buf_size) public pure {
            bytes data = new bytes(10);
            bytes smol_buf = new bytes(buf_size);
            smol_buf.writeBytes(data, 0);
        }
    }"#,
    );

    runtime.function_expect_failure("write_bytes_failure", 9u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: data does not fit into buffer in test.sol"));
}

#[test]
fn math_overflow() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function math_overflow(int8 num) public pure returns (int8) {
            int8 ovf = num + 120;
            return ovf;
        }
    }"#,
    );

    runtime.function_expect_failure("math_overflow", 10u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: math overflow in test.sol"));
}

#[test]
fn require() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function require_test(int8 num) public pure returns (int8) {
            require(num > 10, "sesa");
            return 0;
        }
    }"#,
    );

    runtime.function_expect_failure("require_test", 9u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: sesa require condition failed in test.sol"));
}
#[test]
fn assert() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function assert_test(int8 num) public returns (int8) {
            assert(num > 10);
            return 0;
        }
    }"#,
    );

    runtime.function_expect_failure("assert_test", 9u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: assert failure in test.sol"));
}

#[test]
fn set_storage_bytes_oob() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        bytes public b = hex"0000_00fa";
        function set_storage_bytes() public returns (bytes) {
            bytes sesa = new bytes(1);
            b[5] = sesa[0];
            return sesa;
        }
    }"#,
    );

    runtime.constructor(0, vec![]);
    runtime.function_expect_failure("set_storage_bytes", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: storage index out of bounds in test.sol"));
}

#[test]
fn get_storage_bytes_oob() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        bytes public b = hex"0000_00fa";
        function get_storage_bytes() public returns (bytes) {
            bytes sesa = new bytes(1);
            sesa[0] = b[5];
            return sesa;
        }
    }"#,
    );

    runtime.constructor(0, vec![]);
    runtime.function_expect_failure("get_storage_bytes", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: storage array index out of bounds in test.sol"));
}

#[test]
fn transfor_fails() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
            function transfer_abort() public {
                address a = address(0);
                payable(a).transfer(10);
            }
    }"#,
    );

    runtime.function_expect_failure("transfer_abort", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: value transfer failure in test.sol"));
}

#[test]
fn empty_storage_array_pop() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        uint256[] public arr;
        function pop_empty_storage() public {
            arr.pop();
        }
    }"#,
    );

    runtime.constructor(0, vec![]);
    runtime.function_expect_failure("pop_empty_storage", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: pop from empty storage array in test.sol"));
}

#[test]
fn contract_instantiatoin_fail() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        child public c;
        child public c2;
        constructor() payable {}
        function create_child() public {
                c = new child{value: 900e15, salt: hex"02"}();
                c2 = new child{value: 900e15, salt: hex"02"}();
                uint128 x = address(this).balance;
                print("x = {}".format(x));
        }
    }

    contract child {
        function say_my_name() public pure returns (string memory) {
            print("say_my_name");
            return "child";
        }
    }"#,
    );

    runtime.set_transferred_value(3500);
    runtime.constructor(0, Vec::new());

    runtime.set_transferred_value(0);
    runtime.function_expect_failure("create_child", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: contract creation failed in test.sol:6"));
}

#[test]
fn revert() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
            function i_will_revert() public {
                revert();
            }
    }"#,
    );

    runtime.function_expect_failure("i_will_revert", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: revert encountered in test.sol"));
}

#[test]
fn int_too_large_for_bytes() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function write_integer_failure(uint8 buf_size) public {
            bytes smol_buf = new bytes(buf_size);
            smol_buf.writeUint32LE(350, 20);
        }
    }"#,
    );

    runtime.function_expect_failure("write_integer_failure", 1u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: integer too large to write in buffer in test.sol"));
}

#[test]
fn invalid_instruction() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function invalid_instruction() public pure {
            assembly {
                invalid()
            }
        }
    }"#,
    );

    runtime.function_expect_failure("invalid_instruction", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: reached invalid instruction in test.sol"));
}

#[test]
fn array_index_oob() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function out_of_bounds(uint8 input) public pure returns (uint256) {
            uint256[] a = new uint256[](input);
            return a[20];
        } 
    }"#,
    );

    runtime.function_expect_failure("out_of_bounds", 19u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: array index out of bounds in test.sol"));
}

#[test]
fn truncated_type_overflow() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function trunc_failure(uint128 input) public returns (uint256) {
            uint256[] a = new uint256[](input);
            return a[0];
        }
    }"#,
    );

    runtime.function_expect_failure("trunc_failure", u128::MAX.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: truncated type overflows in test.sol"));
}

#[test]
fn byte_cast_fail() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function byte_cast_failure(uint8 num) public pure returns (bytes) {
            bytes smol_buf = new bytes(num);
            bytes32 b32 = bytes32(smol_buf);
            return b32;
        }
    }"#,
    );

    runtime.function_expect_failure("byte_cast_failure", 33u8.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: bytes cast error in test.sol"));
}

#[test]
fn int_read_oob() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        function read_integer_failure(uint32 offset) public {
            bytes smol_buf = new bytes(1);
            smol_buf.readUint16LE(offset);
        }
    }"#,
    );

    runtime.function_expect_failure("read_integer_failure", 2u32.encode());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: read integer out of bounds in test.sol"));
}

#[test]
fn external_call() {
    let mut runtime = build_solidity(
        r#"contract RuntimeErrors {
        callee public cal;
        constructor() payable {}
        function call_ext() public {
            cal = new callee();
            cal.callee_func{gas: 1e15}();
        }
    }

    contract callee {
        function callee_func() public {
            revert();
        }
    }"#,
    );

    runtime.set_transferred_value(3500);
    runtime.constructor(0, vec![]);
    runtime.function_expect_failure("call_ext", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: external call failed in test.sol"));
}

#[test]
fn errors_() {
    let mut runtime =
        build_solidity(r#"contract RuntimeErrors { function dont_pay_me() public {} }"#);

    runtime.set_transferred_value(1);
    runtime.function_expect_failure("dont_pay_me", Vec::new());
    assert!(runtime
        .debug_buffer()
        .contains("runtime_error: non payable function dont_pay_me received value"));
}
