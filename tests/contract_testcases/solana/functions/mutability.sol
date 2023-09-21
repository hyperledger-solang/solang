contract C {
	bool x;
	int[4] a;

	function test() public view returns (int) {
		return foo()[1];
	}

	function foo() internal returns (int[4] storage) {
		x = true;
		return a;
	}

}

// ---- Expect: diagnostics ----
// error: 6:10-15: function declared 'view' but this expression writes to state