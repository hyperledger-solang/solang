contract foo {
    function test(uint64 x) public returns (bool, uint64) {
        return (true, x * 961748941);
    }
}
