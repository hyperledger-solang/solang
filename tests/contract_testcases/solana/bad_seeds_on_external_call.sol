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

// ----
// error (226-229): type bytes1 found where array of slices expected
// error (271-272): expected 'bytes[]', found integer
// error (296-297): type string found where array of slices expected
// error (321-330): type bytes2 found where array of slices expected
// error (354-355): type bytes found where array of slices expected
// error (394-395): expected 'bytes', found integer
// error (412-417): conversion from struct AccountMeta[1] to bytes not possible
// error (433-444): type bytes found where array bytes expected
