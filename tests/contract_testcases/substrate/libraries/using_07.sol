function foo1() {}
function foo2(int) {}
function foo2(uint) {}
function foo3(int v) returns (int) {
	return v * 3;
}
function foo4(int v, int b) returns (int) {
	return v * 3 + b;
}

contract C {
	using {foo1} for int global;
	using {foo2} for * feh;
	using {foo3} for uint;
	using {foo3, foo4} for int;

	function test(int c) public {
		int a = c.foo3();

		a.foo4(1);
	}
}

// ----
// error (205-209): 'foo1' has no arguments. At least one argument required
// 	note (9-13): definition of 'foo1'
// error (219-225): 'global' on using within contract not permitted
// error (235-239): 'foo2' is an overloaded function
// 	note (28-32): definition of 'foo2'
// 	note (41-61): definition of 'foo2'
// error (247-250): 'feh' not expected, did you mean 'global'?
// error (260-264): function cannot be used since first argument is 'int256' rather than the required 'uint256'
// 	note (73-77): definition of 'foo3'
