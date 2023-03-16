contract Test {
	struct S {
		int foo;
		S[] s;
	}

	function test(int f, S[] ss) public returns (S) {
		return S(f, ss);
	}
}
