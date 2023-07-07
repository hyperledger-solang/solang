contract Foo {
	struct S { S[] s; }
	function decode(bytes b) public pure {
		abi.decode(b, (S));
	}
	function encode() public pure {
		abi.encode(S({ s: new S[](0) }));
	}
}

// ---- Expect: diagnostics ----
// error: 4:3-21: Invalid type 'struct Foo.S': mappings and recursive types cannot be abi decoded or encoded
// error: 7:14-34: Invalid type 'struct Foo.S': mappings and recursive types cannot be abi decoded or encoded
