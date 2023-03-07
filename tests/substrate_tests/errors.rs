// SPDX-License-Identifier: Apache-2.0

use crate::build_solidity_with_options;
use parity_scale_codec::Encode;

#[test]
fn errors() {
    let mut runtime = build_solidity_with_options(
        r#"contract RuntimeErrors {
        bytes b = hex"0000_00fa";
        uint256[] arr;
        child public c;
        child public c2;
        callee public cal;

        constructor() public payable {}

        function print_test(int8 num) public returns (int8) {
            print("Hello world!");
            return num;
        }

        function math_overflow(int8 num) public returns (int8) {
            int8 ovf = num + 120;
            return ovf;
        }

        function require_test(int8 num) public returns (int8) {
            require(num > 10, "sesa");
            return 0;
        }

        // assert failure
        function assert_test(int8 num) public returns (int8) {
            assert(num > 10);
            return 0;
        }

        // storage index out of bounds
        function set_storage_bytes() public returns (bytes) {
            bytes sesa = new bytes(1);
            b[5] = sesa[0];
            return sesa;
        }

        // storage array index out of bounds
        function get_storage_bytes() public returns (bytes) {
            bytes sesa = new bytes(1);
            sesa[0] = b[5];
            return sesa;
        }

        // value transfer failure
        function transfer_abort() public {
            address a = address(0);
            payable(a).transfer(10);
        }

        //  pop from empty storage array
        function pop_empty_storage() public {
            arr.pop();
        }

        // external call failed
        function call_ext() public {
            //cal = new callee();
            cal.callee_func{gas: 1e15}();
        }

        // contract creation failed (contract was deplyed with no value)
        function create_child() public {
            c = new child{value: 900e15, salt:2}();
            c2 = new child{value: 900e15, salt:2}();
            uint128 x = address(this).balance;
            //print("sesa");
            print("x = {}".format(x));

        }

        // non payable function dont_pay_me received value
        function dont_pay_me() public {}

        function pay_me() public payable {
            print("PAYED");
            uint128 x = address(this).balance;
            //print("sesa");
            print("x = {}".format(x));

        }

        function i_will_revert() public {
            revert();
        }

        function write_integer_failure(uint8 buf_size) public {
            bytes smol_buf = new bytes(buf_size);
            smol_buf.writeUint32LE(350, 20);
        }

        function write_bytes_failure(uint8 buf_size) public {
            bytes data = new bytes(10);
            bytes smol_buf = new bytes(buf_size);
            smol_buf.writeBytes(data, 0);
        }

        function read_integer_failure(uint32 offset) public {
            bytes smol_buf = new bytes(1);
            smol_buf.readUint16LE(offset);
        }

        // truncated type overflows
        function trunc_failure(uint128 input) public returns (uint256) {
            uint256[] a = new uint256[](input);
            return a[0];
        }

        function out_of_bounds(uint8 input) public returns (uint256) {
            uint256[] a = new uint256[](input);
            return a[20];
        }

        function invalid_instruction() public {
            assembly {
                invalid()
            }
        }

        function byte_cast_failure(uint8 num) public returns (bytes) {
            bytes smol_buf = new bytes(num);

            //bytes32 b32 = new bytes(num);
            bytes32 b32 = bytes32(smol_buf);
            return b32;
        }
    }

    contract callee {
        constructor() {}

        function callee_func() public {
            revert();
        }
    }

    contract child {
        constructor() {}

        function say_my_name() public pure returns (string memory) {
            print("say_my_name");
            return "child";
        }
    }
    "#,
        false,
        true,
    );

    runtime.function_expect_failure("write_bytes_failure", 9u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: data does not fit into buffer in test.sol:95:22-32,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("math_overflow", 10u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: math overflow in test.sol:16:24-33,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("require_test", 9u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: sesa require condition failed in test.sol:21:31-37,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("assert_test", 9u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: assert failure in test.sol:27:20-28,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("set_storage_bytes", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: storage index out of bounds in test.sol:34:15-16,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("get_storage_bytes", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: storage array index out of bounds in test.sol:41:23-27,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("transfer_abort", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: value transfer failure in test.sol:48:33-35,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("pop_empty_storage", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: pop from empty storage array in test.sol:53:17-20,\n"
    );

    runtime.vm.value = 3500;
    runtime.constructor(0, Vec::new());

    runtime.printbuf.clear();
    runtime.vm.value = 0;
    runtime.function_expect_failure("create_child", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: contract creation failed in test.sol:65:18-52,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("i_will_revert", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: revert encountered in test.sol:84:13-21,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("write_integer_failure", 1u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: integer too large to write in buffer in test.sol:89:22-35,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("invalid_instruction", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: reached invalid instruction in test.sol:116:17-26,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("out_of_bounds", 19u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: array index out of bounds in test.sol:111:20-25,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("trunc_failure", u128::MAX.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: truncated type overflows in test.sol:105:41-46,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("byte_cast_failure", 33u8.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: bytes cast error in test.sol:124:27-44,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("read_integer_failure", 2u32.encode());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: read integer out of bounds in test.sol:100:22-34,\n"
    );

    runtime.printbuf.clear();
    runtime.function_expect_failure("call_ext", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: external call failed in test.sol:59:13-41,\n"
    );

    runtime.printbuf.clear();
    runtime.vm.value = 1;
    runtime.function_expect_failure("dont_pay_me", Vec::new());

    assert_eq!(
        runtime.printbuf,
        "runtime_error: non payable function dont_pay_me received value,\n"
    );
}
