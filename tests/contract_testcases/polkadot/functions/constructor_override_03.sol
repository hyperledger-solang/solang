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

// ---- Expect: diagnostics ----
// warning: 2:20-26: 'public': visibility for constructors is ignored
// warning: 7:20-21: function parameter 'b' is unused
// warning: 7:23-29: 'public': visibility for constructors is ignored
// warning: 8:23-24: function parameter 'b' is unused
// warning: 8:26-32: 'public': visibility for constructors is ignored
// warning: 10:26-27: function parameter 'i' is unused
// warning: 11:25-26: function parameter 's' is unused
// warning: 11:28-34: 'public': visibility for constructors is ignored
