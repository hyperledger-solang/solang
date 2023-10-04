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

// ---- Expect: diagnostics ----
// error: 13:4-7: type bytes1 found where array expected
// error: 15:4-5: expected 'bytes[]', found integer
// error: 16:4-5: type string found where array expected
// error: 17:4-13: type bytes2 found where array expected
// error: 18:4-5: type bytes found where array expected
// error: 20:5-6: expected 'bytes', found integer
// error: 21:5-10: conversion from struct AccountMeta[1] to bytes not possible
// error: 22:4-15: type bytes found where array bytes[] expected
