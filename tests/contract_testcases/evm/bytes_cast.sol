contract C {
	function test1(bytes10 x) public returns (bytes8) {
		return x;
	}

	function test2(bytes10 x) public returns (bytes10) {
		return x;
	}

	function test3(bytes10 x) public returns (bytes12) {
		return x;
	}
}

// ---- Expect: diagnostics ----
// error: 3:10-11: implicit conversion would truncate from bytes10 to bytes8
