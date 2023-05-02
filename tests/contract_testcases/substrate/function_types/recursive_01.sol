contract C {
	struct S {
		int f1;
		S[] f2;
	}

	function foo(S s) public {}
}

// ---- Expect: diagnostics ----
// error: 7:2-26: Recursive parameter not allowed for public or external functions.
