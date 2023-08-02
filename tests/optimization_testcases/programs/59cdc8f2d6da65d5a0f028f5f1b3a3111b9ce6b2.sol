contract Testing {
    struct S {
        int64 f1;
        string f2;
    }

    function test1() public pure returns (bytes memory) {
        S[] memory s = new S[](5);
        return abi.encode(s);
    }

    function test2() public pure returns (bytes memory) {
        string[] memory x = new string[](5);
        return abi.encode(x);
    }
}
