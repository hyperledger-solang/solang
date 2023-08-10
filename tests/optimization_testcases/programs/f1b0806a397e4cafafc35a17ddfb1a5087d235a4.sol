contract Testing {
    function testBytesArray() public pure returns (bytes memory) {
        bytes4[2] memory arr = ["abcd", "efgh"];
        bytes5[] memory vec = new bytes5[](2);
        vec[0] = "12345";
        vec[1] = "67890";
        bytes memory b = abi.encode(arr, vec);
        return b;
    }
}
