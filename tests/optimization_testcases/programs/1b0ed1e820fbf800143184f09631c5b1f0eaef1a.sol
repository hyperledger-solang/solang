contract foo {
    function test() public returns (uint) {
        uint x = .5 * 8;
        return x;
    }

    function test2() public returns (uint) {
        uint x = .4 * 8 + 0.8;
        return x;
    }
}
