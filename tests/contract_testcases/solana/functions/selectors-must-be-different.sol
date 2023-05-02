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
// ---- Expect: diagnostics ----
// error: 6:2-22: function 'f2' selector is the same as function 'f1'
// 	note 3:2-22: definition of function 'f1'
// error: 9:2-22: function 'f3' selector is the same as function 'f1'
// 	note 3:2-22: definition of function 'f1'
// warning: 13:13-14: c is already defined as a contract name
// 	note 1:1-10:2: location of previous definition
// error: 17:2-22: function 'f1' selector is the same as function 'c'
// 	note 13:13-14: definition of function 'c'
// error: 22:2-16: function 'f1' selector must be 8 bytes rather than 1 bytes
// error: 31:2-22: function 'f2' selector is the same as function 'f1'
// 	note 29:2-22: definition of function 'f1'
// error: 39:2-22: function 'f3' selector is the same as function 'f1'
// 	note 36:2-22: definition of function 'f1'
