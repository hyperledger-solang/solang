contract Testing {
    function testThis(bytes memory bb) public pure {
        (uint32 a, uint16[][] memory vec, int64 b) = abi.decode(
            bb,
            (uint32, uint16[][], int64)
        );
        assert(a == 99);
        assert(vec[0][0] == 99);
        assert(vec[0][1] == 20);
        assert(vec[1][0] == 15);
        assert(vec[1][1] == 88);
        assert(b == -755);
    }
}
