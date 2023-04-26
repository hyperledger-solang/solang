
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

// ----
// error (21-23): X1 is already defined as an enum
// 	note (6-8): location of previous definition
// error (27-30): 'function', 'error', or 'event' expected
// error (37-40): 'function', 'error', or 'event' expected
// error (71-73): X2 is already defined as an error
// 	note (53-55): location of previous definition
// warning (102-104): X2 is already defined as an error
// 	note (53-55): location of previous definition
// error (137-138): error 'X3' has duplicate field name 'X'
// 	note (125-130): location of previous declaration of 'X'
// error (175-196): mapping type is not permitted as error field
// error (209-210): type 'X' not found
// error (214-217): 'function', 'error', or 'event' expected
// error (225-229): 'function', 'error', or 'event' expected
