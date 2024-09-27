// https://github.com/hyperledger-solang/solang/issues/860
contract c {
	function test() public pure returns (uint256 ret1) {
		uint256[3][] vec;
		vec.push([1, 2, 3]);
		return vec[1][1];
	}
}

// ---- Expect: diagnostics ----
