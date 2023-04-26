contract C {
	constructor foo() public {}
    function foo() public pure {}
}

// ----
// warning (32-38): 'public': visibility for constructors is ignored
// error (46-72): Non unique function or constructor name 'foo'
// 	note (14-38): previous declaration of 'foo'
