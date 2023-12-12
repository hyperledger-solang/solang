int constant C1 = LEN1;
int constant C3 = foo();

contract c {
	int constant C2 = LEN1;

	bool[C1] var1;
	bool[C2] var22;
}

// ---- Expect: diagnostics ----
// error: 1:19-23: 'LEN1' not found
// error: 2:19-24: cannot call function in constant expression
// error: 5:20-24: 'LEN1' not found
