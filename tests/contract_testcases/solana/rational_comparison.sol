// Ensure that rational comparisons are not permitted
contract c {
	function foo1(uint64 a, uint64 b) public returns (bool) {
		return (a/b) >= 0.05;
	}
	function foo2(uint64 a, uint64 b) public returns (bool) {
		return 002.2 > a;
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

// ----
// error (135-148): cannot use rational numbers with '>=' operator
// error (221-230): cannot use rational numbers with '>' operator
// error (303-312): cannot use rational numbers with '!=' or '==' operator
// error (385-386): expression not allowed in constant rational number expression
// error (467-483): cannot use rational numbers with '<=' operator
// error (556-570): cannot use rational numbers with '!=' or '==' operator
