contract c {
	uint32 state;

	function f(uint32 count) public {
		uint[] memory array = new uint[](count <<= 1);
	}
	function g() public {
		uint[] memory array = new uint[](state <<= 1);
	}
	function h(uint32 count) public {
		uint[] memory array = new uint[](i());
	}
	function i() public {}
}

// ---- Expect: diagnostics ----
// warning: 4:20-25: function parameter 'count' is unused
// warning: 5:17-22: local variable 'array' is unused
// warning: 8:17-22: local variable 'array' is unused
// error: 11:36-39: new dynamic array should have an unsigned length argument
