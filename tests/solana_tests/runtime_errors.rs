// SPDX-License-Identifier: Apache-2.0

use crate::{build_solidity, BorshToken};
use num_bigint::BigInt;

#[test]
fn runtime_errors() {
    let mut vm = build_solidity(
        r#"
contract RuntimeErrors {
    bytes b = hex"0000_00fa";
    uint256[] arr;
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

    function revert_with_message() public pure {
        revert("I reverted!");
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
    );

    vm.set_program(0);
    let data_account = vm.initialize_data_account();
    vm.function("new")
        .accounts(vec![("dataAccount", data_account)])
        .call();

    let mut _res = vm
        .function("math_overflow")
        .arguments(&[BorshToken::Int {
            width: 8,
            value: BigInt::from(10u8),
        }])
        .must_fail();
    assert_eq!(
        vm.logs,
        "runtime_error: math overflow in test.sol:19:20-29,\n"
    );
    vm.logs.clear();

    _res = vm
        .function("require_test")
        .arguments(&[BorshToken::Int {
            width: 256,
            value: BigInt::from(9u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: sesa require condition failed in test.sol:25:27-33,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("get_storage_bytes")
        .accounts(vec![("dataAccount", data_account)])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: storage array index out of bounds in test.sol:45:19-23,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("set_storage_bytes")
        .accounts(vec![("dataAccount", data_account)])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: storage index out of bounds in test.sol:38:11-12,\n"
    );
    vm.logs.clear();

    _res = vm
        .function("read_integer_failure")
        .arguments(&[BorshToken::Uint {
            width: 32,
            value: BigInt::from(2u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: read integer out of bounds in test.sol:71:18-30,\n"
    );
    vm.logs.clear();

    _res = vm
        .function("trunc_failure")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(u128::MAX),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: truncated type overflows in test.sol:76:37-42,\n"
    );
    vm.logs.clear();

    _res = vm.function("invalid_instruction").must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: reached invalid instruction in test.sol:87:13-22,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("pop_empty_storage")
        .accounts(vec![("dataAccount", data_account)])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: pop from empty storage array in test.sol:51:9-12,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("write_bytes_failure")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(9u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: data does not fit into buffer in test.sol:66:18-28,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("assert_test")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(9u8),
        }])
        .must_fail();
    println!("{}", vm.logs);
    assert_eq!(
        vm.logs,
        "runtime_error: assert failure in test.sol:31:16-24,\n"
    );
    vm.logs.clear();

    _res = vm
        .function("out_of_bounds")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(19u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: array index out of bounds in test.sol:82:16-21,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("write_integer_failure")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(1u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: integer too large to write in buffer in test.sol:60:18-31,\n"
    );

    vm.logs.clear();

    _res = vm
        .function("byte_cast_failure")
        .arguments(&[BorshToken::Uint {
            width: 256,
            value: BigInt::from(33u8),
        }])
        .must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: bytes cast error in test.sol:95:23-40,\n"
    );

    vm.logs.clear();

    _res = vm.function("i_will_revert").must_fail();

    assert_eq!(
        vm.logs,
        "runtime_error: unspecified revert encountered in test.sol:55:9-17,\n"
    );

    vm.logs.clear();

    _res = vm.function("revert_with_message").must_fail();
    assert_eq!(
        vm.logs,
        "runtime_error: I reverted! revert encountered in test.sol:100:9-30,\n"
    );
    assert!(vm.return_data.is_none());
}
