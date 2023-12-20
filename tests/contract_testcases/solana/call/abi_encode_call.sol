contract abi_encode_call {
    int[2] ints;

    function test1() public {
        bytes memory bs = abi.encodeCall(other.foo, 1);
    }

    function test2() public {
        bytes memory bs = abi.encodeCall(other.foo, (1, true));
    }

    function test3() public {
        bytes memory bs = abi.encodeCall(other.baz, (1, 2, ints));
    }

    function test4() public {
        bytes memory bs = abi.encodeCall(other.foo, (1, 2), 3);
    }
}

contract other {
    function foo(int foo, int bar) public {}

    function baz(int foo, int bar, int[2] storage) internal {}
}

// ---- Expect: diagnostics ----
// error: 5:27-55: function takes 2 arguments, 1 provided
// 	note 22:5-45: definition of foo
// error: 9:57-61: conversion from bool to int256 not possible
// error: 13:42-47: function is not public or external
// 	note 24:5-63: definition of baz
// error: 17:27-63: function expects 2 arguments, 3 provided
// warning: 22:22-25: declaration of 'foo' shadows function
// 	note 22:14-17: previous declaration of function
// warning: 24:22-25: declaration of 'foo' shadows function
// 	note 22:14-17: previous declaration of function
