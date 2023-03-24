contract Foo {
	struct S { S[] s; }
	function decode(bytes b) public pure {
		abi.decode(b, (S));
	}
	function encode() public pure {
		abi.encode(S({ s: new S[](0) }));
	}
}
