abstract contract c {
	@selector([1])
	function f1() public {}
	// error: selector must be unique
	@selector([1])
	function f2() public {}
	// error: selector must be unique
	@selector([1])
	function f3() public {}
}

contract d {
	int public c;

	// error: selector is the same as c
	@selector([0x13, 0xfb, 0xd7, 0x25, 0xfe, 0xff, 0x6e, 0x10])
	function f1() public {}
}

contract e {
	// error: selector must be 8 bytes
	@selector([1])
	function f1() public {}
}

contract f {
	// error: selectors are the same
	@selector([0x41, 0x42, 0x43, 0x44, 0xca, 0xff, 0xee, 0x00])
	function f1() public {}
	@selector([0x41, 0x42, 0x43, 0x44, 0xca, 0xff, 0xee, 0x00])
	function f2() public {}
	function f3() public {}
}

contract g {
	function f1() public {}
	// error: selector for f3 matches f1
	@selector([0x1b, 0x49, 0x4c, 0xee, 0x9c, 0x54, 0x1e, 0x94])
	function f3() public {}
}