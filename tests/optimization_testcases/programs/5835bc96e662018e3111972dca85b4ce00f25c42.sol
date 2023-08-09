contract foo {
    function test() public returns (uint) {
        uint x = 4.8 % 0.2;
        return x;
    }
}
