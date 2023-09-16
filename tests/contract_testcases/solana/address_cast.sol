contract c {
	function foo() public {
		// We have code that cast address type to ref address
		// in fn sema::cast(). Ensure that this does not cause
		// address values to be assignable.
		address(0) = address(this);
	}
}

// ---- Expect: diagnostics ----
// error: 6:3-13: expression is not assignable
