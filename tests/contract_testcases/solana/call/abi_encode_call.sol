contract abi_encode_call {
    function test1() public {
        bytes bs = abi.encodeCall(other.foo, 1);
    }

    function test2() public {
        bytes bs = abi.encodeCall(other.foo, 1, true);
    }
}

contract other {
    function foo(int foo, int bar) public {}
}