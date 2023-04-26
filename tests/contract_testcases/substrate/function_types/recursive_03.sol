contract Test {
	struct S {
		int foo;
		S[] s;
	}

	function test(int f, S[] ss) public returns (S) {
		return S(f, ss);
	}
}

// ----
// error (53-100): Recursive parameter not allowed for public or external functions.
