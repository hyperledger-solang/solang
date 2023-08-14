contract Testing {
    function testStruct(string memory rr) public pure returns (bytes memory) {
        bytes memory b1 = abi.encode(rr);
        return b1;
    }
}
