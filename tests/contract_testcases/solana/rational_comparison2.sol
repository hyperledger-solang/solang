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

