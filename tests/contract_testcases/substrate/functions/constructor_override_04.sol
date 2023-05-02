contract C {
	constructor foo() public {}
    function foo() public pure {}
}

// ---- Expect: diagnostics ----
// warning: 2:20-26: 'public': visibility for constructors is ignored
// error: 3:5-31: Non unique function or constructor name 'foo'
// 	note 2:2-26: previous declaration of 'foo'
