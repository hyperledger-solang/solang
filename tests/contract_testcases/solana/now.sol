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
	function j() public returns (int) {
		int now = 5;
		return now;
	}
}
