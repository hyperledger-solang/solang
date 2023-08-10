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

    noPadStruct[] test_vec_1;

    function addData() public {
        noPadStruct memory mm = noPadStruct(1623, 43279);
        test_vec_1.push(mm);
        mm.a = 41234;
        mm.b = 98375;
        test_vec_1.push(mm);
        mm.a = 945;
        mm.b = 7453;
        test_vec_1.push(mm);
    }

    function encodeStruct() public view returns (bytes memory) {
        PaddedStruct memory ss = PaddedStruct(1, 3, "there_is_padding_here");
        bytes memory b = abi.encode(test_vec_1[2], ss);
        return b;
    }

    function primitiveStruct() public view returns (bytes memory) {
        int32[4] memory mem_vec = [int32(1), -298, 3, -434];
        noPadStruct[2] memory str_vec = [noPadStruct(1, 2), noPadStruct(3, 4)];
        bytes memory b1 = abi.encode(test_vec_1, mem_vec, str_vec);
        return b1;
    }

    function primitiveDynamicArray() public view returns (bytes memory) {
        noPadStruct[] memory str_vec = new noPadStruct[](2);
        str_vec[0] = noPadStruct(5, 6);
        str_vec[1] = noPadStruct(7, 8);
        bytes memory b2 = abi.encode(str_vec);
        return b2;
    }
}
