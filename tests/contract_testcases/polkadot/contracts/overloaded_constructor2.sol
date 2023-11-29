contract S {
	constructor(int32 a) {}
	constructor(int64 a) {}

	function p() public {}
}

contract T {
	function test1() public {
		new S(1);
	}

	function test2() public {
		new S({a: 1});
	}

	function test3() public {
		new S({b: 1});
	}
}

// ---- Expect: diagnostics ----
// warning: 2:20-21: function parameter 'a' is unused
// warning: 3:20-21: function parameter 'a' is unused
// error: 10:3-11: constructor can be resolved to multiple functions
// 	note 2:2-25: candidate constructor
// 	note 3:2-25: candidate constructor
// error: 14:3-16: function call can be resolved to multiple functions
// 	note 2:2-25: candidate function
// 	note 3:2-25: candidate function
// error: 18:3-16: cannot find overloaded constructor which matches signature
// error: 18:3-16: missing argument 'a' to constructor
// 	note 2:2-23: definition of constructor
// 	note 2:2-25: candidate constructor
// error: 18:3-16: missing argument 'a' to constructor
// 	note 3:2-23: definition of constructor
// 	note 3:2-25: candidate constructor
