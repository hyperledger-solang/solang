// RUN: --target polkadot --emit cfg

contract C {
// BEGIN-CHECK: C::C::function::f1
	function f1(string a) public returns (string) {
		return string.concat("", a, "");
		// CHECK: return (arg #0)
	}

// BEGIN-CHECK: C::C::function::f2
	function f2(string a) public returns (string) {
		return string.concat("b", "ar", ": ", a, "");
		// CHECK: return (builtin Concat ((alloc string uint32 5 "bar: "), (arg #0)))
	}
}
