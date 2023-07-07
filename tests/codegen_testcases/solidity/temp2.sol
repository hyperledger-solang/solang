// RUN: --target polkadot --emit cfg
contract c {
    event f(bool);

	function test() public returns (int32) {
		int32 x = 104;
        emit f(true);
        x = x + 1;
        emit f(false);
// CHECK: return int32 105
		return x;
	}
}
