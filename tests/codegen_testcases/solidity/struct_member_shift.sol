// RUN: --target polkadot --emit cfg
contract c {
    struct S {
        uint256 a;
    }
	function test1(S memory s) public pure returns (uint256) {
// CHECK: ty:uint256 %b = (uint256 2 << (load (struct (arg #0) field 0)))
		uint256 b = 2 << s.a;
		return b;
	}

    function test2(S memory s) public pure returns (uint256) {
// CHECK: ty:uint256 %b = ((load (struct (arg #0) field 0)) << uint256 2)
		uint256 b = s.a << 2;
		return b;
	}

    function test3(S memory s) public pure returns (uint256) {
// CHECK: ty:uint256 %b = (uint256 2 >> (load (struct (arg #0) field 0)))
		uint256 b = 2 >> s.a;
		return b;
	}

    function test4(S memory s) public pure returns (uint256) {
// CHECK: ty:uint256 %b = ((load (struct (arg #0) field 0)) >> uint256 2)
		uint256 b = s.a >> 2;
		return b;
	}
}
