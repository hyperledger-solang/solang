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
