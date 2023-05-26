contract c {
	function eq() public returns (bool) {
		return 1.1 == 1.0;
	}

	function ne() public returns (bool) {
		return 1.1 != 1.0;
	}

	function lt() public {
		require(1.0 < 1.1);
	}

	function le(bool a) public {
		require(a && 0.1 <= 1.1);
	}

	function gt(bool a) public {
		if (1 > 0.5) { }
	}

	function gt(int a) public returns (bool) {
		return a > 1.1;
	}

	function ge(bool a) public {
		gt(1 >= 1.02);
	}
}


// ---- Expect: diagnostics ----
// error: 3:10-20: cannot use rational numbers with '==' operator
// error: 7:10-20: cannot use rational numbers with '!=' operator
// error: 11:11-20: cannot use rational numbers with '<' operator
// error: 15:16-26: cannot use rational numbers with '<=' operator
// error: 19:7-14: cannot use rational numbers with '>' operator
// error: 23:10-17: cannot use rational numbers with '>' operator
// error: 27:6-15: cannot use rational numbers with '>=' operator
