abstract contract A {
	constructor foo() public {}
	constructor () {}
}

contract C is A {
	constructor (bool b) public {}
	constructor foo(bool b) public {}
    function foo() public pure {}
    function foo(uint256 i) public pure {}
	constructor foo(string s) public {}
}

// ----
// warning (41-47): 'public': visibility for constructors is ignored
// warning (110-111): function parameter 'b' has never been read
// warning (113-119): 'public': visibility for constructors is ignored
// warning (145-146): function parameter 'b' has never been read
// warning (148-154): 'public': visibility for constructors is ignored
// warning (217-218): function parameter 'i' has never been read
// warning (259-260): function parameter 's' has never been read
// warning (262-268): 'public': visibility for constructors is ignored
