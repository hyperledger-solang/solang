contract Testing {
    function extraElement(bytes memory buffer) public pure {
        (int64[] memory vec, int32 g) = abi.decode(buffer, (int64[], int32));

        assert(vec[1] == 0);
        assert(g == 3);
    }
}
