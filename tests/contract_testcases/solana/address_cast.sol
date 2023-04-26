contract c {
	function foo() public {
		// We have code that cast address type to ref address
		// in fn sema::cast(). Ensure that this does not cause
		// address values to be assignable.
		address(0) = tx.program_id;
	}
}

// ----
// error (191-201): expression is not assignable
