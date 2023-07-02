
contract asserts {
	int64 public var = 1;

	function test_assert() public {
		var = 2;
		assert(false);
	}

	function test_assert_rpc() public pure returns (int64) {
		revert("I refuse");
	}
}
