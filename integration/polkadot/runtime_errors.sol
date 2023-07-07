contract RuntimeErrors {
    bytes b = hex"0000_00fa";
    uint256[] arr;
    child_runtime_errors public c;

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
    function call_ext(callee_error e) public {
        e.callee_func();
    }

    // contract creation failed (contract was deplyed with no value)
    function create_child() public {
        c = new child_runtime_errors();
    }

    // non payable function dont_pay_me received value
    function dont_pay_me() public {}

    function pay_me() public payable {}

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
        bytes32 b32 = bytes32(smol_buf);
        return b32;
    }
}

contract callee_error {
    constructor() {}

    function callee_func() public {
        revert();
    }
}

contract child_runtime_errors {
    constructor() payable {}

    function say_my_name() public pure returns (string memory) {
        return "child_runtime_errors";
    }
}
