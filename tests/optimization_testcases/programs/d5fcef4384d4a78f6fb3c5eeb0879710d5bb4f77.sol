contract foo {
    function test(uint64 x) public returns (uint64, uint) {
        return (x * 961748941, 2.5 + 3.5 - 1);
    }
}
