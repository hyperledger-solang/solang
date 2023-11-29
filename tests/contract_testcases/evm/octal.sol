contract C {
	int public test1 = 00;
	int public test2 = 0;
	int public test3 = 01e1;
	int public test4 = 09.1 * 10;
	int public test5 = -00;
	int public test6 = -0;
	int public test7 = -01e1;
	int public test8 = -09.1 * 10;
}

// ---- Expect: diagnostics ----
// error: 2:21-23: leading zeros not permitted, can be confused with octal
// error: 4:21-25: leading zeros not permitted, can be confused with octal
// error: 5:21-25: leading zeros not permitted, can be confused with octal
// error: 6:21-24: leading zeros not permitted, can be confused with octal
// error: 8:21-26: leading zeros not permitted, can be confused with octal
// error: 9:22-26: leading zeros not permitted, can be confused with octal
