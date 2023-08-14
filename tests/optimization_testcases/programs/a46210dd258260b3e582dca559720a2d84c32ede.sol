contract Testing {
    struct PaddedStruct {
        uint128 a;
        uint8 b;
        bytes32 c;
    }

    function multiDimStruct(bytes memory buffer) public pure {
        (PaddedStruct[2][3][] memory vec, int16 g) = abi.decode(
            buffer,
            (PaddedStruct[2][3][], int16)
        );

        assert(vec.length == 1);

        assert(vec[0][0][0].a == 56);
        assert(vec[0][0][0].b == 1);
        assert(vec[0][0][0].c == "oi");

        assert(vec[0][0][1].a == 78);
        assert(vec[0][0][1].b == 6);
        assert(vec[0][0][1].c == "bc");

        assert(vec[0][1][0].a == 89);
        assert(vec[0][1][0].b == 4);
        assert(vec[0][1][0].c == "sn");

        assert(vec[0][1][1].a == 42);
        assert(vec[0][1][1].b == 56);
        assert(vec[0][1][1].c == "cn");

        assert(vec[0][2][0].a == 23);
        assert(vec[0][2][0].b == 78);
        assert(vec[0][2][0].c == "fr");

        assert(vec[0][2][1].a == 445);
        assert(vec[0][2][1].b == 46);
        assert(vec[0][2][1].c == "br");

        assert(g == -90);
    }

    function multiDimInt(bytes memory buffer) public pure {
        uint16[4][2][] memory vec = abi.decode(buffer, (uint16[4][2][]));

        assert(vec.length == 2);

        assert(vec[0][0][0] == 1);
        assert(vec[0][0][1] == 2);
        assert(vec[0][0][2] == 3);
        assert(vec[0][0][3] == 4);

        assert(vec[0][1][0] == 5);
        assert(vec[0][1][1] == 6);
        assert(vec[0][1][2] == 7);
        assert(vec[0][1][3] == 8);

        assert(vec[1][0][0] == 9);
        assert(vec[1][0][1] == 10);
        assert(vec[1][0][2] == 11);
        assert(vec[1][0][3] == 12);

        assert(vec[1][1][0] == 13);
        assert(vec[1][1][1] == 14);
        assert(vec[1][1][2] == 15);
        assert(vec[1][1][3] == 16);
    }

    function uniqueDim(bytes memory buffer) public pure {
        uint16[] memory vec = abi.decode(buffer, (uint16[]));

        assert(vec.length == 5);

        assert(vec[0] == 9);
        assert(vec[1] == 3);
        assert(vec[2] == 4);
        assert(vec[3] == 90);
        assert(vec[4] == 834);
    }
}
