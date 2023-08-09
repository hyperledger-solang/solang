contract Testing {
    function testLongerBuffer(bytes memory buffer) public view {
        uint64 a = abi.decode(buffer, (uint64));

        assert(a == 4);
    }
}
