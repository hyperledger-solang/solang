contract Testing {
    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    function getThis() public pure returns (bytes memory) {
        PaddedStruct memory a = PaddedStruct(56, 1, "oi");
        PaddedStruct memory b = PaddedStruct(78, 6, "bc");
        PaddedStruct memory c = PaddedStruct(89, 4, "sn");
        PaddedStruct memory d = PaddedStruct(42, 56, "cn");
        PaddedStruct memory e = PaddedStruct(23, 78, "fr");
        PaddedStruct memory f = PaddedStruct(445, 46, "br");

        PaddedStruct[2][3] memory vec = [[a, b], [c, d], [e, f]];

        PaddedStruct[2][3][] memory arr2 = new PaddedStruct[2][3][](1);
        arr2[0] = vec;

        uint16 g = 5;
        bytes memory b1 = abi.encode(arr2, g);
        return b1;
    }

    function multiDim() public pure returns (bytes memory) {
        uint16[4][2] memory vec = [[uint16(1), 2, 3, 4], [uint16(5), 6, 7, 8]];

        uint16[4][2][] memory simple_arr = new uint16[4][2][](1);
        simple_arr[0] = vec;

        bytes memory b = abi.encode(simple_arr);
        return b;
    }

    function uniqueDim() public pure returns (bytes memory) {
        uint16[] memory vec = new uint16[](5);
        vec[0] = 9;
        vec[1] = 3;
        vec[2] = 4;
        vec[3] = 90;
        vec[4] = 834;
        bytes memory b = abi.encode(vec);
        return b;
    }
}
