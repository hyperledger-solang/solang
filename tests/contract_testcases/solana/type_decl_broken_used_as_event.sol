type X is int;

contract c {
	function f() public {
		emit X();
	}
}

// ----
// error (59-60): 'X' is an user type
