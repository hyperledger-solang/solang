contract Testing {
    function returnedString(
        bytes memory buffer
    ) public pure returns (string memory) {
        string memory s = abi.decode(buffer, (string));
        return s;
    }
}
