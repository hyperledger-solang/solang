// RUN: --target polkadot --emit cfg
contract c {
	function test() public pure returns (int32) {
		int32 x = 104;
        int32 t = x;
        x += 1;
// CHECK: return int32 104
		return t;
	}
}
