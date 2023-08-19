contract c {
	function test() public {
		uint x = -80 ** 512;
		int y = 80 << 100000;
		int z = -80 << 100000;
	}
}

// ---- Expect: diagnostics ----
// error: 3:12-22: value is too large to fit into type uint256
// error: 4:11-23: value is too large to fit into type int256
// error: 5:11-24: value is too large to fit into type int256
