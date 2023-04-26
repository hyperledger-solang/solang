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

// ----
// warning (48-53): function parameter 'count' has never been read
// warning (80-85): local variable 'array' has been assigned, but never read
// warning (155-160): local variable 'array' has been assigned, but never read
// error (261-264): new dynamic array should have an unsigned length argument
