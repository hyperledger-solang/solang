contract C {
	function foo(int256 a) internal pure returns (int256) {
		return a;
	}
}

library L {
	function bar(int256 a) internal pure returns (int256) {
		return a;
	}
}

library Lib {
	function baz(int256 a, bool b) internal pure returns (int256) {
		if (b) {
			return 1;
		} else {
			return a;
		}
	}
	using {L.bar, baz} for int256;
}

library Lib2 {
	using {L.foo.bar, C.foo} for int256;
}

// ---- Expect: diagnostics ----
// error: 25:15-18: 'foo' not found
// error: 25:20-25: 'C.foo' is not a library function
// 	note 2:2-55: definition of C.foo
