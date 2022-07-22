import "solana";

contract c {
	address zero = address(0);

	function test1() public {
		bytes instr = new bytes(0);
		AccountMeta[1] metas;
		string s = "abc";
		bytes d = hex"abcd";

		zero.call{accounts: metas, seeds: [
			"a", // should be slice
			[ "b"],  // ok
			1, // should be slice
			s, // should be slice
			hex"f1f2", // should be slice
			d, // should be slice
			[d], // ok
			[1], // not ok
			[metas], // not ok
			[ [ "a" ] ]  // should be slice
		]}(instr);
	}
}
