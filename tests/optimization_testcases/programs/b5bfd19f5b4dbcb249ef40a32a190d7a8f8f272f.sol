contract Testing {
    function testStringVector(
        bytes memory buffer
    ) public pure returns (string[] memory) {
        string[] memory vec = abi.decode(buffer, (string[]));
        return vec;
    }
}
