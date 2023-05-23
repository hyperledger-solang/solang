contract c {
	uint16 public x = 0x10000 * 10 - 0x9ffff;

	// When resolving the argument to require, we first 
	// resolve it with ResolveTo::Unknown. Make sure this
	// works correctly for integer expressions
	function f(uint i, bytes4 b) public pure {
		require(i < 2**225);
		require(i > 255+255);
		require(i >= 127*127);
		require(i <= (2**127)*2);
		require(b != 0);
		require(b == 0);
	}
}

// ---- Expect: diagnostics ----
