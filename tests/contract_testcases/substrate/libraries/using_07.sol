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
