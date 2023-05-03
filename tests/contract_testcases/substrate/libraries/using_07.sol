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

// ---- Expect: diagnostics ----
// error: 12:9-13: 'foo1' has no arguments. At least one argument required
// 	note 1:10-14: definition of 'foo1'
// error: 12:23-29: 'global' on using within contract not permitted
// error: 13:9-13: 'foo2' is an overloaded function
// 	note 2:10-14: definition of 'foo2'
// 	note 3:1-21: definition of 'foo2'
// error: 13:21-24: 'feh' not expected, did you mean 'global'?
// error: 14:9-13: function cannot be used since first argument is 'int256' rather than the required 'uint256'
// 	note 4:10-14: definition of 'foo3'
