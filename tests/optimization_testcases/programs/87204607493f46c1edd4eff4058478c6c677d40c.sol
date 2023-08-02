contract Testing {
    function getThis() public pure returns (bytes memory) {
        string memory a = "coffe";
        bytes memory b = "tea";
        bytes memory c = abi.encode(a, b);
        return c;
    }
}
