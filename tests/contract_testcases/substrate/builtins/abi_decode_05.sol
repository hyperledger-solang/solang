contract Foo {
	struct S { S[] s; }
	function decode(bytes b) public pure {
		abi.decode(b, (S));
	}
	function encode() public pure {
		abi.encode(S({ s: new S[](0) }));
	}
}

// ----
// error (78-96): Invalid type 'struct Foo.S': mappings and recursive types cannot be abi decoded or encoded
// error (147-167): Invalid type 'struct Foo.S': mappings and recursive types cannot be abi decoded or encoded
