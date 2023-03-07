
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
