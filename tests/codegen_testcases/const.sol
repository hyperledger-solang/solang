// RUN: --emit cfg
contract c {
	function test() public pure returns (int32) {
		int32 x = 102;
// CHECK: return int32 102
		return x;
	}
}
