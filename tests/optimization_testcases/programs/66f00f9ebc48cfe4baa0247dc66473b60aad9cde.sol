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

    function twoStructs(bytes memory buffer) public pure {
        (NoPadStruct memory a, PaddedStruct memory b) = abi.decode(
            buffer,
            (NoPadStruct, PaddedStruct)
        );
        assert(a.a == 945);
        assert(a.b == 7453);
        assert(b.a == 1);
        assert(b.b == 3);
        assert(b.c == "there_is_padding_here");
    }

    function fixedArrays(bytes memory buffer) public pure {
        (
            int32[4] memory a,
            NoPadStruct[2] memory b,
            NoPadStruct[] memory c
        ) = abi.decode(buffer, (int32[4], NoPadStruct[2], NoPadStruct[]));

        assert(a[0] == 1);
        assert(a[1] == -298);
        assert(a[2] == 3);
        assert(a[3] == -434);

        assert(b[0].a == 1);
        assert(b[0].b == 2);
        assert(b[1].a == 3);
        assert(b[1].b == 4);

        assert(c.length == 3);
        assert(c[0].a == 1623);
        assert(c[0].b == 43279);
        assert(c[1].a == 41234);
        assert(c[1].b == 98375);
        assert(c[2].a == 945);
        assert(c[2].b == 7453);
    }

    function primitiveDynamic(bytes memory buffer) public pure {
        NoPadStruct[] memory vec = abi.decode(buffer, (NoPadStruct[]));

        assert(vec.length == 2);
        assert(vec[0].a == 5);
        assert(vec[0].b == 6);
        assert(vec[1].a == 7);
        assert(vec[1].b == 8);
    }
}
