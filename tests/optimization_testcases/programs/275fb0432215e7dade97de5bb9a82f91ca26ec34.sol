contract Testing {
    function testByteArrays(bytes memory buffer) public view {
        (bytes4[2] memory arr, bytes5[] memory vec) = abi.decode(
            buffer,
            (bytes4[2], bytes5[])
        );

        assert(arr[0] == "abcd");
        assert(arr[1] == "efgh");

        assert(vec.length == 2);
        assert(vec[0] == "12345");
        assert(vec[1] == "67890");
    }
}
