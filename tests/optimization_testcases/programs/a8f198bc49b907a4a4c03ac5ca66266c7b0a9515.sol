contract Testing {
    function testThis() public pure returns (bytes) {
        uint16[][] memory vec;
        vec = new uint16[][](2);
        vec[0] = new uint16[](2);
        vec[1] = new uint16[](2);
        vec[0][0] = 90;
        vec[0][1] = 31;
        vec[1][0] = 52;
        vec[1][1] = 89;
        uint32 gg = 99;
        int64 tt = -190;
        bytes b = abi.encode(gg, vec, tt);
        return b;
    }
}
