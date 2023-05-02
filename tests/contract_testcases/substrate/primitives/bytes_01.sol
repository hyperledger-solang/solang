contract Foo {
	function foo() public pure {
		bytes3 b = bytes3(0x0000AA);
		uint16 d  = 0x00bffc; // is allowed
		bytes2 c = 0x00bffc; // not allowed
	}
}

// ---- Expect: diagnostics ----
// error: 5:14-22: hex literal 0x00bffc must be 4 digits for type 'bytes2'
