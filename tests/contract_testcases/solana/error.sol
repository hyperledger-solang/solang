
contract error {
	error X();

	function foo(error x) public {
		
	}
}

// ----
// warning (25-26): error 'X' has never been used
// warning (32-60): function can be declared 'pure'
// warning (51-52): function parameter 'x' has never been read
