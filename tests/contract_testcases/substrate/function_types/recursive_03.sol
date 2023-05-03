contract Test {
	struct S {
		int foo;
		S[] s;
	}

	function test(int f, S[] ss) public returns (S) {
		return S(f, ss);
	}
}

// ---- Expect: diagnostics ----
// error: 7:2-49: Recursive parameter not allowed for public or external functions.
