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
// ---- Expect: diagnostics ----
// error: 3:20-48: function takes 2 arguments, 1 provided
// error: 7:49-53: conversion from bool to int256 not possible
// warning: 12:22-25: declaration of 'foo' shadows function
// 	note 12:14-17: previous declaration of function
