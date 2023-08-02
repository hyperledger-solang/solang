contract Testing {
    function testLongerBuffer(bytes memory buffer) public view {
        (uint64 a, uint32[3] memory b) = abi.decode(
            buffer,
            (uint64, uint32[3])
        );

        assert(a == 4);
        assert(b[0] == 1);
        assert(b[1] == 2);
        assert(b[2] == 3);
    }
}
