contract c {
	/// @return meh
	/// @return feh
	function foo() public returns (int) {
		return 1;
	}
}

// ---- Expect: diagnostics ----
// error: 3:7-14: duplicate tag '@return'
