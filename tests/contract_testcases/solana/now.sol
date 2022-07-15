contract c {
	struct S { int f1; }
	function f(S s) public returns (int) {
		return now.s;
	}
	function g() public {
		now();
	}
	function h() public returns (uint) {
		return now;
	}
	function j() public returns (bool) {
		return now > 102;
	}
	function k() public returns (int) {
		int now = 5;
		return now;
	}
	event LockRecord(address a,uint n,uint256 m);
	function foo() public {
		emit LockRecord(address(this), now, 34);
		emit LockRecord({a: address(this), n: now, m: 34});
	}
}
