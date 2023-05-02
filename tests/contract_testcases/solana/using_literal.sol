function double(int x) pure returns (int) { return x * 2; }

using {double} for int;

contract C {
	function foo() pure public returns (int) {
		return 42.double();
	}
}

// ---- Expect: diagnostics ----
