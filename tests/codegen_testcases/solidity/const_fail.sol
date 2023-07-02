// RUN: --target polkadot --emit cfg
contract c {
	function divide_zero() public pure returns (uint32) {
		uint32 x = 2;
		x = x / 0;
// FAIL: divide by zero
		return x;
	}

}
