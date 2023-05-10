function double(int x) pure returns (int) { return x * 2; }

interface C {
	using {double} for int;
}

// ---- Expect: diagnostics ----
// error: 4:2-24: using for not permitted in interface
