
contract error {
	error X();

	function foo(error x) public {
		
	}
}

// ---- Expect: diagnostics ----
// warning: 3:8-9: error 'X' has never been used
// warning: 5:2-30: function can be declared 'pure'
// warning: 5:21-22: function parameter 'x' has never been read
