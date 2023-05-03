contract Test {
	struct S {
		int foo;
		S[][2] s;
	}

	function test(int f, S[][2] ss) public returns (S) {
		return S(f, ss);
	}
}


// ---- Expect: diagnostics ----
// error: 7:2-52: Recursive parameter not allowed for public or external functions.
