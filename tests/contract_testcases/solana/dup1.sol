@program_id("F1ipperKF9EfD821ZbbYjS319LXYiBmjhzkkf5a26rC")
contract C {
	function foo(int8 a, int8 b) external {}
	function foo(int64 a, int8 b) external {}
}

contract D {
	function test1() external {
		C.foo(1, 2);
	}

	function test2() external {
		C.foo(x, y);
	}
}

// ---- Expect: diagnostics ----
// warning: 3:20-21: function parameter 'a' is unused
// warning: 3:28-29: function parameter 'b' is unused
// warning: 4:21-22: function parameter 'a' is unused
// warning: 4:29-30: function parameter 'b' is unused
// error: 9:3-14: function call can be resolved to multiple functions
// 	note 3:2-42: candidate function
// 	note 4:2-43: candidate function
// error: 13:9-10: 'x' not found
// error: 13:12-13: 'y' not found
