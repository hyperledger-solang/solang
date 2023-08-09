contract OneSwapToken {
    function testIt(
        uint256[] calldata mixedAddrVal
    ) public pure returns (uint256, uint256) {
        uint256 a = mixedAddrVal[0] << 2;
        uint256 b = mixedAddrVal[1] >> 2;
        return (a, b);
    }
}
