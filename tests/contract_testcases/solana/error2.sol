
enum X1 { a }
error X1();
foo X1();
a.b X1();
error X2(int bar);
enum X2 { b }


contract C {
	error X2(int bar);
	error X3(int X, bool X);
	error X4(int X, bool);
	error X5(mapping (int => bool));
	error X6(X);
	meh X7();
	a[1] X8(int);
}

// ---- Expect: diagnostics ----
// error: 3:7-9: X1 is already defined as an enum
// 	note 2:6-8: location of previous definition
// error: 4:1-4: 'function', 'error', or 'event' expected
// error: 5:1-4: 'function', 'error', or 'event' expected
// error: 7:6-8: X2 is already defined as an error
// 	note 6:7-9: location of previous definition
// warning: 11:8-10: X2 is already defined as an error
// 	note 6:7-9: location of previous definition
// error: 12:23-24: error 'X3' has duplicate field name 'X'
// 	note 12:11-16: location of previous declaration of 'X'
// error: 14:11-32: mapping type is not permitted as error field
// error: 15:11-12: type 'X' not found
// error: 16:2-5: 'function', 'error', or 'event' expected
// error: 17:2-6: 'function', 'error', or 'event' expected
