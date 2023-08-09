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

    function getThis() public pure returns (bytes memory) {
        noPadStruct memory a = noPadStruct(1238, 87123);
        bytes memory b = abi.encode(a);
        return b;
    }

    function getThat() public pure returns (bytes memory) {
        PaddedStruct memory a = PaddedStruct(12998, 240, "tea_is_good");
        bytes memory b = abi.encode(a);
        return b;
    }
}
