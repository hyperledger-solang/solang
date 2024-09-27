// https://github.com/hyperledger-solang/solang/issues/859
contract c {
	uint256[3][] s_vec;
	function test() public returns (uint256 ret1) {
		s_vec.push([1, 2, 3]);
		return s_vec[1][1];
	}
}

// ---- Expect: diagnostics ----
