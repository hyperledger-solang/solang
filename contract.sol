pragma solidity 0;

contract maxer {
	function max(uint64 x, uint64 y) public pure returns (uint64) {
		if (x > y) {
			return x;
		} else {
			return y;
		}
	}
}
