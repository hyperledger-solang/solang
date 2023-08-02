contract Testing {
    function testStringAndBytes() public pure returns (bytes memory) {
        string str = "coffee";
        bytes memory b = new bytes(9);
        b.writeString(str, 0);
        bytes memory g = "tea";
        b.writeBytes(g, 6);
        return b;
    }
}
