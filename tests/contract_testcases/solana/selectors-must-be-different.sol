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
	function f1() selector=hex"c3da42b8" public {}
}

contract e {
	// error: selector must be 4 bytes
	function f1() selector=hex"01" public {}
}

contract f {
	// error: selectors are the same
	function f1() selector=hex"41424344" public {}
	function f2() selector=hex"41424344" public {}
	function f3() public {}
}

contract g {
	function f1() public {}
	// error: selector for f3 matches f1
	function f3() selector=hex"c27fc305" public {}
}