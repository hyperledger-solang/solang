contract C {
	function f1(bytes1 a, bytes b) public returns (bytes c) { c = a + b; }
	function f2(bytes a, bytes2 b) public returns (bytes c) { c = a + b; }
	function f3(bytes a, bytes b) public returns (bytes c) { c = a + b; }
	function f4(string a, string b) public returns (string c) { c = a + b; }
	function f(string a, bytes b) public returns (bytes c) { c = a + b; }
}

// ---- Expect: diagnostics ----
// error: 2:64-69: concatenate bytes using the builtin bytes.concat(a, b)
// error: 3:64-69: concatenate bytes using the builtin bytes.concat(a, b)
// error: 4:63-68: concatenate bytes using the builtin bytes.concat(a, b)
// error: 5:66-71: concatenate string using the builtin string.concat(a, b)
// error: 6:63-64: expression of type string not allowed
