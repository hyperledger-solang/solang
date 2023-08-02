contract Testing {
    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    function decodeComplex(bytes memory buffer) public view {
        NonConstantStruct[] memory vec = abi.decode(
            buffer,
            (NonConstantStruct[])
        );

        assert(vec.length == 2);

        assert(vec[0].a == 897);
        assert(vec[0].b[0] == "tea");
        assert(vec[0].b[1] == "coffee");

        assert(vec[1].a == 74123);
        assert(vec[1].b[0] == "cortado");
        assert(vec[1].b[1] == "cappuccino");
    }

    function dynamicArray(bytes memory buffer) public view {
        int16[] memory vec = abi.decode(buffer, (int16[]));

        assert(vec.length == 3);

        assert(vec[0] == -90);
        assert(vec[1] == 5523);
        assert(vec[2] == -89);
    }

    function decodeMultiDim(bytes memory buffer) public view {
        int8[2][3] memory vec = abi.decode(buffer, (int8[2][3]));

        print("{}".format(vec[0][1]));
        assert(vec[0][0] == 1);
        assert(vec[0][1] == 2);
        assert(vec[1][0] == 4);
        assert(vec[1][1] == 5);
        assert(vec[2][0] == 6);
        assert(vec[2][1] == 7);
    }
}
