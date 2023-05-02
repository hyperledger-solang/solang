contract C {
	struct S {
		int f1;
		S[] f2;
	}
}

// ---- Expect: diagnostics ----
