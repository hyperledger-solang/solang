contract C {
	function foo(int) public {}
	function foo(int a, bool b) public {}
	function foo(bool b, int a) public {}
	function test1() public {
		foo({a: true, b: 1});
	}

	function test2() public {
		foo({a: 1, b: true});
	}

	function test3() public {
		foo({a: moo, b: meh});
	}
}

contract B is C {
	function test1() public {
		B.foo({a: true, b: 1});
	}

	function test2() public {
		B.foo({a: 1, b: true});
	}

	function test3(C c) public {
		c.foo({a: true, b: 1});
	}

	function test4(C c) public {
		c.foo({a: 1, b: true});
	}

	function test5(C c) public {
		c.bar({a: 1, b: true});
	}

	function test6(C c) public {
		c.bar({a: 1, b: x});
	}
}

// ---- Expect: diagnostics ----
// error: 5:2-25: function 'test1' with this signature already defined
// 	note 19:2-25: previous definition of function 'test1'
// error: 6:3-23: cannot find overloaded function which matches signature
// 	note 2:2-29: candidate function
// error: 6:3-23: cannot find overloaded function which matches signature
// 	note 3:2-39: candidate function
// error: 6:3-23: cannot find overloaded function which matches signature
// 	note 4:2-39: candidate function
// error: 6:3-23: function cannot be called with named arguments as 1 of its parameters do not have names
// 	note 2:2-26: definition of foo
// 	note 2:2-29: candidate function
// error: 6:11-15: conversion from bool to int256 not possible
// 	note 3:2-39: candidate function
// error: 6:11-15: conversion from bool to int256 not possible
// 	note 4:2-39: candidate function
// error: 6:20-21: expected 'bool', found integer
// 	note 3:2-39: candidate function
// error: 6:20-21: expected 'bool', found integer
// 	note 4:2-39: candidate function
// error: 9:2-25: function 'test2' with this signature already defined
// 	note 23:2-25: previous definition of function 'test2'
// error: 10:3-23: function call can be resolved to multiple functions
// 	note 3:2-39: candidate function
// 	note 4:2-39: candidate function
// error: 14:11-14: 'moo' not found
// error: 14:19-22: 'meh' not found
// error: 20:3-25: cannot find overloaded function which matches signature
// 	note 2:2-29: candidate function
// error: 20:3-25: cannot find overloaded function which matches signature
// 	note 3:2-39: candidate function
// error: 20:3-25: cannot find overloaded function which matches signature
// 	note 4:2-39: candidate function
// error: 20:3-25: function cannot be called with named arguments as 1 of its parameters do not have names
// 	note 2:2-26: definition of foo
// 	note 2:2-29: candidate function
// error: 20:13-17: conversion from bool to int256 not possible
// 	note 3:2-39: candidate function
// error: 20:13-17: conversion from bool to int256 not possible
// 	note 4:2-39: candidate function
// error: 20:22-23: expected 'bool', found integer
// 	note 3:2-39: candidate function
// error: 20:22-23: expected 'bool', found integer
// 	note 4:2-39: candidate function
// error: 24:3-25: function call can be resolved to multiple functions
// 	note 3:2-39: candidate function
// 	note 4:2-39: candidate function
// error: 28:3-25: cannot find overloaded function which matches signature
// 	note 2:2-29: candidate function
// error: 28:3-25: cannot find overloaded function which matches signature
// 	note 3:2-39: candidate function
// error: 28:3-25: cannot find overloaded function which matches signature
// 	note 4:2-39: candidate function
// error: 28:3-25: function cannot be called with named arguments as 1 of its parameters do not have names
// 	note 2:2-26: definition of foo
// 	note 2:2-29: candidate function
// error: 28:13-17: conversion from bool to int256 not possible
// 	note 3:2-39: candidate function
// error: 28:13-17: conversion from bool to int256 not possible
// 	note 4:2-39: candidate function
// error: 28:22-23: expected 'bool', found integer
// 	note 3:2-39: candidate function
// error: 28:22-23: expected 'bool', found integer
// 	note 4:2-39: candidate function
// error: 32:3-25: function call can be resolved to multiple functions
// 	note 3:2-39: candidate function
// 	note 4:2-39: candidate function
// error: 36:3-25: contract 'C' does not have function 'bar'
// error: 40:19-20: 'x' not found
