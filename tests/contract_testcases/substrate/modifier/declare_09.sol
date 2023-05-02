contract c {
	function bar() public pure {
		_; _();
	}
	function _() private pure {}
}

// ---- Expect: diagnostics ----
