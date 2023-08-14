contract Testing {
    int16[] vec_1;

    function addData() public {
        vec_1.push(-90);
        vec_1.push(5523);
        vec_1.push(-89);
    }

    struct NonConstantStruct {
        uint64 a;
        string[] b;
    }

    function encodeComplex() public returns (bytes memory) {
        string[] vec_2 = new string[](2);
        vec_2[0] = "tea";
        vec_2[1] = "coffee";
        NonConstantStruct[] arr = new NonConstantStruct[](2);
        arr[0] = NonConstantStruct(897, vec_2);

        string[] vec_3 = new string[](2);
        vec_3[0] = "cortado";
        vec_3[1] = "cappuccino";
        arr[1] = NonConstantStruct(74123, vec_3);
        return abi.encode(arr);
    }

    function encodeArray() public view returns (bytes memory) {
        bytes memory b = abi.encode(vec_1);
        return b;
    }

    function multiDimArrays() public pure returns (bytes memory) {
        int8[2][3] memory vec = [[int8(1), 2], [int8(4), 5], [int8(6), 7]];
        bytes memory b = abi.encode(vec);
        return b;
    }
}
