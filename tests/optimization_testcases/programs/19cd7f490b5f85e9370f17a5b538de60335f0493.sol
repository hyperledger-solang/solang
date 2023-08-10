contract Testing {
    function testStringOut() public pure returns (bytes memory) {
        bytes memory b = new bytes(4);
        string memory str = "cappuccino";
        b.writeString(str, 0);
        return b;
    }
}
