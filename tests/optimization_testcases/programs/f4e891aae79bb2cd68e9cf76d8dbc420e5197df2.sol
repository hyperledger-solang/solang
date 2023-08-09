contract c {
    bytes foo;

    function set_foo(bytes bs) public {
        foo = bs;
    }

    function foo_length() public returns (uint32) {
        return foo.length;
    }

    function set_foo_offset(uint32 index, bytes1 b) public {
        foo[index] = b;
    }

    function get_foo_offset(uint32 index) public returns (bytes1) {
        return foo[index];
    }
}
