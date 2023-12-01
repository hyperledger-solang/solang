// Ensure that rational comparisons are not permitted
contract c {
	function foo1(uint64 a, uint64 b) public returns (bool) {
		return (a/b) >= 0.05;
	}
	function foo2(uint64 a, uint64 b) public returns (bool) {
		return 2.2 > a;
	}
	function foo3(uint64 a, uint64 b) public returns (bool) {
		return 1 == 0.05;
	}
	function foo4(uint64 a, uint64 b) public returns (bool) {
		return a*2.1 < b;
	}
	function foo5(uint64 a, uint64 b) public returns (bool) {
		return (a << b) <= 0.05;
	}
	function foo6(uint64 a, uint64 b) public returns (bool) {
		return 1.2 != (a ^ b);
	}
}

// ---- Expect: diagnostics ----
// error: 4:10-23: cannot use rational numbers with '>=' operator
// error: 7:10-17: cannot use rational numbers with '>' operator
// error: 10:10-19: cannot use rational numbers with '==' operator
// error: 13:10-11: expression not allowed in constant rational number expression
// error: 16:10-26: cannot use rational numbers with '<=' operator
// error: 19:10-24: cannot use rational numbers with '!=' operator
