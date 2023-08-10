contract Testing {
    function testArrayAssign(bytes memory buffer) public pure {
        int32[2][] memory vec = abi.decode(buffer, (int32[2][]));

        assert(vec.length == 2);

        assert(vec[0][0] == 0);
        assert(vec[0][1] == 1);
        assert(vec[1][0] == 2);
        assert(vec[1][1] == -3);
    }
}
