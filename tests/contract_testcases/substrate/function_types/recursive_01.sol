contract C {
	struct S {
		int f1;
		S[] f2;
	}

	function foo(S s) public {}
}

// ----
// error (50-74): Recursive parameter not allowed for public or external functions.
