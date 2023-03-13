// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity_with_options, BorshToken};
use num_bigint::BigInt;

#[test]
fn runtime_errors() {
    let mut vm = build_solidity_with_options(
        r#"
contract RuntimeErrors {
    bytes b = hex"0000_00fa";
    uint256[] arr;
    child public c;
    child public c2;

    constructor() {}

    function print_test(int8 num) public returns (int8) {
        print("Hello world!");

        require(num > 10, "sesa");
        assert(num > 10);

        int8 ovf = num + 120;
        print("x = {}".format(ovf));
        return ovf;
    }

    function math_overflow(int8 num) public returns (int8) {
        int8 ovf = num + 120;
        print("x = {}".format(ovf));
        return ovf;
    }

    function require_test(int256 num) public returns (int8) {
        require(num > 10, "sesa");
        return 0;
    }

    // assert failure
    function assert_test(int256 num) public returns (int8) {
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

    //  pop from empty storage array
    function pop_empty_storage() public {
        arr.pop();
    }


    // contract creation failed
    function create_child() public {
        address a = address(0);
        c = new child{address: a}();
        //c2 = new child();
        uint128 x = address(this).balance;
        //print("sesa");
        print("x = {}".format(x));
        
    }

    function i_will_revert() public {
        revert();
    }

    function write_integer_failure(uint256 buf_size) public {
        bytes smol_buf = new bytes(buf_size);
        smol_buf.writeUint32LE(350, 20);
    }

    function write_bytes_failure(uint256 buf_size) public {
        bytes data = new bytes(10);
        bytes smol_buf = new bytes(buf_size);
        smol_buf.writeBytes(data, 0);
    }

    function read_integer_failure(uint32 offset) public {
        bytes smol_buf = new bytes(1);
        smol_buf.readUint16LE(offset);
    }

    // truncated type overflows
    function trunc_failure(uint256 input) public returns (uint256[]) {
        uint256[] a = new uint256[](input);
        return a;
    }

    function out_of_bounds(uint256 input) public returns (uint256) {
        uint256[] a = new uint256[](input);
        return a[20];
    }

    function invalid_instruction() public {
        assembly {
            invalid()
        }
    }

    function byte_cast_failure(uint256 num) public returns (bytes) {
        bytes smol_buf = new bytes(num);

        //bytes32 b32 = new bytes(num);
        bytes32 b32 = bytes32(smol_buf);
        return b32;
    }

}

@program_id("Crea1hXZv5Snuvs38GW2SJ1vJQ2Z5uBavUnwPwpiaDiQ")
contract child {
    constructor() {}

    function say_my_name() public pure returns (string memory) {
        print("say_my_name");
        return "child";
    }
}

contract calle_contract {
    constructor() {}

    function calle_contract_func() public {
        revert();
    }
}

 "#,
        true,
        true,
    );

    vm.set_program(0);
    vm.constructor(&[]);

    let mut _res = vm.function_must_fail(
        "math_overflow",
        &[BorshToken::Int {
            width: 8,
            value: BigInt::from(10u8),
        }],
    );
    assert_eq!(
        vm.logs,
        "runtime_error: math overflow in test.sol:22:20-29,\n"
    );
    vm.logs.clear();

    _res = vm.function_must_fail(
        "require_test",
        &[BorshToken::Int {
            width: 256,
            value: BigInt::from(9u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: sesa require condition failed in test.sol:28:27-33,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail("get_storage_bytes", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: storage array index out of bounds in test.sol:48:19-23,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail("set_storage_bytes", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: storage index out of bounds in test.sol:41:11-12,\n"
    );
    vm.logs.clear();

    _res = vm.function_must_fail(
        "read_integer_failure",
        &[BorshToken::Uint {
            width: 32,
            value: BigInt::from(2u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: read integer out of bounds in test.sol:86:18-30,\n"
    );
    vm.logs.clear();

    _res = vm.function_must_fail(
        "trunc_failure",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(u128::MAX),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: truncated type overflows in test.sol:91:37-42,\n"
    );
    vm.logs.clear();

    _res = vm.function_must_fail("invalid_instruction", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: reached invalid instruction in test.sol:102:13-22,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail("pop_empty_storage", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: pop from empty storage array in test.sol:54:9-12,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail(
        "write_bytes_failure",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(9u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: data does not fit into buffer in test.sol:81:18-28,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail(
        "assert_test",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(9u8),
        }],
    );
    println!("{}", vm.logs);
    assert_eq!(
        vm.logs,
        "runtime_error: assert failure in test.sol:34:16-24,\n"
    );
    vm.logs.clear();

    _res = vm.function_must_fail(
        "out_of_bounds",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(19u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: array index out of bounds in test.sol:97:16-21,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail(
        "write_integer_failure",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(1u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: integer too large to write in buffer in test.sol:75:18-31,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail(
        "byte_cast_failure",
        &[BorshToken::Uint {
            width: 256,
            value: BigInt::from(33u8),
        }],
    );

    assert_eq!(
        vm.logs,
        "runtime_error: bytes cast error in test.sol:110:23-40,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail("i_will_revert", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: revert encountered in test.sol:70:9-15,\n"
    );

    vm.logs.clear();

    _res = vm.function_must_fail("create_child", &[]);

    assert_eq!(
        vm.logs,
        "runtime_error: contract creation failed in test.sol:61:13-36,\n"
    );
    vm.logs.clear();
}
