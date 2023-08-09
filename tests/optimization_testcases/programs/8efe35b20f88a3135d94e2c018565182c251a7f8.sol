struct Sector {
    uint8[] mclass;
    uint8[13] _calldata;
}

contract Testing {
    function testBytesArray() public pure returns (bytes memory) {
        uint8[13] x;
        for (uint8 i = 0; i < 13; i++) x[i] = 19 * i;
        Sector s = Sector(new uint8[](0), x);
        bytes memory b = abi.encode(s);
        return b;
    }
}
