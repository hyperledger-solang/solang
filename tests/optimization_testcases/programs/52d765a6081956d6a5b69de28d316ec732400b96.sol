contract Testing {
    function testStringAndBytes(bytes memory buffer) public view {
        (string memory a, bytes memory b) = abi.decode(buffer, (string, bytes));

        assert(a == "coffee");
        assert(b == "tea");
    }
}
