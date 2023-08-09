contract Testing {
    struct NoPadStruct {
        uint32 a;
        uint32 b;
    }

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    function testNoPadStruct(bytes memory buffer) public pure {
        NoPadStruct memory str = abi.decode(buffer, (NoPadStruct));
        assert(str.a == 1238);
        assert(str.b == 87123);
    }

    function testPaddedStruct(bytes memory buffer) public pure {
        PaddedStruct memory str = abi.decode(buffer, (PaddedStruct));
        assert(str.a == 12998);
        assert(str.b == 240);
        assert(str.c == "tea_is_good");
    }
}
