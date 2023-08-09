contract Testing {
    function testBytesOut() public pure returns (bytes memory) {
        bytes memory b = new bytes(9);
        bytes memory g = "tea";
        b.writeBytes(g, 30);
        return b;
    }
}
