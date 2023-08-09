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

    string[] string_vec;
    NonConstantStruct to_encode;

    function testStruct() public returns (bytes memory) {
        noPadStruct memory noPad = noPadStruct(89123, 12354);
        PaddedStruct memory padded = PaddedStruct(988834, 129, "tea_is_good");
        string_vec.push("tea");
        string_vec.push("coffee");

        to_encode = NonConstantStruct(890234, string_vec, noPad, padded);

        bytes memory b1 = abi.encode(to_encode);
        return b1;
    }
}
