contract C {
	enum E { a, b, c }

	function test1() public {
		type(int);
	}

	function test2() public {
		int a = type(int);
	}

	function test3() public {
		assert(type(E).min == 0);
		assert(type(E).max == 2);
	}

	function test4() public {
		int a = type().min;
	}

	function test5() public {
		int a = type(int, bool).min;
	}

	function test6() public {
		int a = type(bool).min;
	}

	function test7() public {
		E e = E(3);
	}
}

// ---- Expect: diagnostics ----
// error: 9:11-20: function or method does not return a value
// error: 18:11-17: missing type argument to type() operator
// error: 22:11-26: type() operator takes a single argument
// error: 26:11-25: type 'bool' does not have type function min
// warning: 30:9-13: enum enum C.E has no value with ordinal 3
