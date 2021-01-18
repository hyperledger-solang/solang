// RUN: --emit cfg
contract c {
// BEGIN-CHECK: c::test
	function test() public pure returns (int32) {
		int32 x = 102;
// CHECK: return int32 102
		return x;
	}

// BEGIN-CHECK: c::add
	function add() public pure returns (int32) {
		int32 x = 5;
		x += 3;
// CHECK: return int32 8
		return x;
	}

// BEGIN-CHECK: c::power
	function power() public pure returns (uint32) {
		uint32 x = 2;
		x = x**4;
// CHECK: return uint32 16
		return x;
	}
}
