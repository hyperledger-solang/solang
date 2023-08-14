contract Testing {
    struct noPadStruct {
        uint32 a;
        uint32 b;
    }

    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    struct NonConstantStruct {
        uint64 a;
        string[] b;
        noPadStruct noPad;
        PaddedStruct pad;
    }

    function testStruct(bytes memory buffer) public pure {
        NonConstantStruct memory str = abi.decode(buffer, (NonConstantStruct));
        assert(str.a == 890234);
        assert(str.b.length == 2);
        assert(str.b[0] == "tea");
        assert(str.b[1] == "coffee");
        assert(str.noPad.a == 89123);
        assert(str.noPad.b == 12354);
        assert(str.pad.a == 988834);
        assert(str.pad.b == 129);
        assert(str.pad.c == "tea_is_good");
    }
}
