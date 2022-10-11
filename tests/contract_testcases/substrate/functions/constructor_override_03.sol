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
