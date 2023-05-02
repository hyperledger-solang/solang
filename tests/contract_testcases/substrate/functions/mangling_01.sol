contract Foo {
	function f_bool() public pure {}

	function f(bool foo) public pure {}
	function f(uint foo) public pure {}
}

// ---- Expect: diagnostics ----
// error: 4:2-34: mangling the symbol of overloaded function 'f' with signature 'f(bool)' results in a new symbol 'f_bool' but this symbol already exists
// 	note 2:2-31: this function declaration conflicts with mangled name
