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
// ----
// error (76-104): function takes 2 arguments, 1 provided
// error (191-195): conversion from bool to int256 not possible
// warning (245-248): declaration of 'foo' shadows function
// 	note (237-240): previous declaration of function
