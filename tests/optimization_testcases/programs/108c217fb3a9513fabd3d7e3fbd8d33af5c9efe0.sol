contract Testing {
    function wrongNumber(bytes memory buffer) public view {
        int64[5] memory vec = abi.decode(buffer, (int64[5]));

        assert(vec[1] == 0);
    }
}
