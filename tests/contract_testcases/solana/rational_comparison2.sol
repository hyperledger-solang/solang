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


// ----
// error (61-71): cannot use rational numbers with '!=' or '==' operator
// error (125-135): cannot use rational numbers with '!=' or '==' operator
// error (175-184): cannot use rational numbers with '<' operator
// error (236-246): cannot use rational numbers with '<=' operator
// error (289-296): cannot use rational numbers with '>' operator
// error (359-366): cannot use rational numbers with '>' operator
// error (407-416): cannot use rational numbers with '>=' operator
