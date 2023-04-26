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
// ----
// error (115-135): function 'f2' selector is the same as function 'f1'
// 	note (39-59): definition of function 'f1'
// error (191-211): function 'f3' selector is the same as function 'f1'
// 	note (39-59): definition of function 'f1'
// warning (243-244): c is already defined as a contract name
// 	note (0-216): location of previous definition
// error (346-366): function 'f1' selector is the same as function 'c'
// 	note (243-244): definition of function 'c'
// error (423-437): function 'f1' selector must be 8 bytes rather than 1 bytes
// error (661-681): function 'f2' selector is the same as function 'f1'
// 	note (575-595): definition of function 'f1'
// error (851-871): function 'f3' selector is the same as function 'f1'
// 	note (727-747): definition of function 'f1'
