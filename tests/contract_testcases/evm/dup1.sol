contract C {
	function foo(int8 a, int8 b) public {}
	function foo(int64 a, int8 b) public {}
	function bar() public {
		foo(1, 2);
	}
}

contract D is C {
	function test1() public {
		C.foo(1, 2);
	}

	function test2(C c) public {
		c.foo(1, 2);
	}

	function test3(C c) public {
		c.foo(1);
	}

	function test4(C c) public {
		c.foo(x, y);
	}
}

// ---- Expect: diagnostics ----
// warning: 2:20-21: function parameter 'a' is unused
// warning: 2:28-29: function parameter 'b' is unused
// warning: 3:21-22: function parameter 'a' is unused
// warning: 3:29-30: function parameter 'b' is unused
// error: 5:3-12: function call can be resolved to multiple functions
// 	note 2:2-40: candidate function
// 	note 3:2-41: candidate function
// error: 11:3-14: function call can be resolved to multiple functions
// 	note 2:2-40: candidate function
// 	note 3:2-41: candidate function
// error: 15:3-14: function call can be resolved to multiple functions
// 	note 2:2-40: candidate function
// 	note 3:2-41: candidate function
// error: 19:3-11: cannot find overloaded function which matches signature
// error: 19:3-11: function expects 2 arguments, 1 provided
// 	note 2:2-37: candidate function
// error: 19:3-11: function expects 2 arguments, 1 provided
// 	note 3:2-38: candidate function
// error: 23:9-10: 'x' not found
// error: 23:12-13: 'y' not found
