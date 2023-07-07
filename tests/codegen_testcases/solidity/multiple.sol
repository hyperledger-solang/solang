// RUN: --target polkadot --emit cfg
contract c {
    event f(bool);

	function test(bool a) public pure returns (int32) {
        int32 x;
        if (a) {
            x = 10 * 5;
        } else {
            x = 45 + 5;
        }
        // both reaching definitions should eval to 50
// CHECK: return int32 50
		return x;
	}
}
