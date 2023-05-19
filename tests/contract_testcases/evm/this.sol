function test1() returns (address) {
	return address(this);
}

function test2(int this) returns (int) {
	return this * 3;
}

contract that {
	// We can shadow this with another variable
	function foo(int this, int super) public pure returns (int) {
		return this + super;
	}
}
// ---- Expect: diagnostics ----
// error: 2:17-21: this not allowed outside contract
// warning: 5:20-24: 'this' shadows name of a builtin
// warning: 11:19-23: 'this' shadows name of a builtin
// warning: 11:29-34: 'super' shadows name of a builtin
