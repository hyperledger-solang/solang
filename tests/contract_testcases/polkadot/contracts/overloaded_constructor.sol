contract S {
	constructor(int) {}
	constructor(int a, bool b) {}
	constructor(bool b, int a) {}

	function p() public {}
}

contract T {
	function test1() public {
		new S();
	}

	function test2() public {
		new S(true);
	}

	function test3() public {
		new S({});
	}

	function test4() public {
		new S({a: 1, b: true});
	}

}

// ---- Expect: diagnostics ----
// warning: 3:18-19: function parameter 'a' is unused
// warning: 3:26-27: function parameter 'b' is unused
// warning: 4:19-20: function parameter 'b' is unused
// warning: 4:26-27: function parameter 'a' is unused
// error: 11:3-10: cannot find overloaded constructor which matches signature
// error: 11:3-10: constructor expects 1 arguments, 0 provided
// 	note 2:2-21: candidate constructor
// error: 11:3-10: constructor expects 2 arguments, 0 provided
// 	note 3:2-31: candidate constructor
// error: 11:3-10: constructor expects 2 arguments, 0 provided
// 	note 4:2-31: candidate constructor
// error: 15:3-14: cannot find overloaded constructor which matches signature
// error: 15:3-14: constructor expects 2 arguments, 1 provided
// 	note 3:2-31: candidate constructor
// error: 15:3-14: constructor expects 2 arguments, 1 provided
// 	note 4:2-31: candidate constructor
// error: 15:9-13: conversion from bool to int256 not possible
// 	note 2:2-21: candidate constructor
// error: 19:3-12: cannot find overloaded constructor which matches signature
// error: 19:3-12: constructor cannot be called with named arguments as 1 of its parameters do not have names
// 	note 2:2-19: definition of constructor
// 	note 2:2-21: candidate constructor
// error: 19:3-12: constructor expects 2 arguments, 0 provided
// 	note 3:2-29: definition of constructor
// 	note 3:2-31: candidate constructor
// error: 19:3-12: constructor expects 2 arguments, 0 provided
// 	note 4:2-29: definition of constructor
// 	note 4:2-31: candidate constructor
// error: 23:3-25: can be resolved to multiple constructors
// 	note 3:2-31: candidate constructor
// 	note 4:2-31: candidate constructor
