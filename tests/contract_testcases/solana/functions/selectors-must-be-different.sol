abstract contract c {
	function f1() selector=hex"01" public {}
	// error: selector must be unique
	function f2() selector=hex"01" public {}
	// error: selector must be unique
	function f3() selector=hex"01" public {}
}

contract d {
	int public c;

	// error: selector is the same as c
	function f1() selector=hex"13fbd725feff6e10" public {}
}

contract e {
	// error: selector must be 8 bytes
	function f1() selector=hex"01" public {}
}

contract f {
	// error: selectors are the same
	function f1() selector=hex"41424344caffee00" public {}
	function f2() selector=hex"41424344caffee00" public {}
	function f3() public {}
}

contract g {
	function f1() public {}
	// error: selector for f3 matches f1
	function f3() selector=hex"1b494cee9c541e94" public {}
}