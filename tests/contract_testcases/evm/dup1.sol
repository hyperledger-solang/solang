contract C {
	function foo(int8 a, int8 b) public {}
	function foo(int64 a, int8 b) public {}
	function bar() public {
		foo(1, 2);
	}
}

// ---- Expect: diagnostics ----
// warning: 2:20-21: function parameter 'a' is unused
// warning: 2:28-29: function parameter 'b' is unused
// warning: 3:21-22: function parameter 'a' is unused
// warning: 3:29-30: function parameter 'b' is unused
// error: 5:3-12: function call can be resolved to multiple functions
// 	note 2:2-37: candidate function
// 	note 3:2-38: candidate function
