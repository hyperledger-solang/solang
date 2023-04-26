contract Foo {
	function f_bool() public pure {}

	function f(bool foo) public pure {}
	function f(uint foo) public pure {}
}

// ----
// error (51-83): mangling the symbol of overloaded function 'f' with signature 'f(bool)' results in a new symbol 'f_bool' but this symbol already exists
// 	note (16-45): this function declaration conflicts with mangled name
