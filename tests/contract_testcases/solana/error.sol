
contract error {
	error X();

	function foo() public {
		
	}
}

// ---- Expect: diagnostics ----
// warning: 3:8-9: error 'X' has never been used
// warning: 5:2-23: function can be declared 'pure'
