contract fixed_bytes_subscript_not_assign {
    bytes32 x;
    function storage_test() public {
        x[1] = 2;
    }
    function memory_test(bytes32 y) public {
        y[1] = 2;
    }
}