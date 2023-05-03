type X is int;

contract c {
	function f() public {
		emit X();
	}
}

// ---- Expect: diagnostics ----
// error: 5:8-9: 'X' is an user type
